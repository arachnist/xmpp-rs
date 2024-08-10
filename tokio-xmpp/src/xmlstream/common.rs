// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::borrow::Cow;
use std::io;

use futures::{ready, Sink, SinkExt, Stream, StreamExt};

use bytes::{Buf, BytesMut};

use tokio::io::{AsyncBufRead, AsyncWrite};

use xso::{
    exports::rxml::{self, writer::TrackNamespace, xml_ncname, Event, Namespace},
    FromEventsBuilder, FromXml, Item,
};

use xmpp_parsers::ns::STREAM as XML_STREAM_NS;

pin_project_lite::pin_project! {
    // NOTE: due to limitations of pin_project_lite, the field comments are
    // no doc comments. Luckily, this struct is only `pub(super)` anyway.
    #[project = RawXmlStreamProj]
    pub(super) struct RawXmlStream<Io> {
        // The parser used for deserialising data.
        #[pin]
        parser: rxml::AsyncReader<Io>,

        // The writer used for serialising data.
        writer: rxml::writer::Encoder<rxml::writer::SimpleNamespaces>,

        // The default namespace to declare on the stream header.
        stream_ns: &'static str,

        // Buffer containing serialised data which will then be sent through
        // the inner `Io`. Sending that serialised data happens in
        // `poll_ready` and `poll_flush`, while appending serialised data
        // happens in `start_send`.
        tx_buffer: BytesMut,

        // This signifies the limit at the point of which the Sink will
        // refuse to accept more data: if the `tx_buffer`'s size grows beyond
        // that high water mark, poll_ready will return Poll::Pending until
        // it has managed to flush enough data down the inner writer.
        //
        // Note that poll_ready will always attempt to progress the writes,
        // which further reduces the chance of hitting this limit unless
        // either the underlying writer gets stuck (e.g. TCP connection
        // breaking in a timeouty way) or a lot of data is written in bulk.
        // In both cases, the backpressure created by poll_ready returning
        // Pending is desirable.
        //
        // However, there is a catch: We don't assert this condition
        // in `start_send` at all. The reason is that we cannot suspend
        // serialisation of an XSO in the middle of writing it: it has to be
        // written in one batch or you have to start over later (this has to
        // do with the iterator state borrowing the data and futures getting
        // cancelled e.g. in tokio::select!). In order to facilitate
        // implementing a `Sink<T: AsXml>` on top of `RawXmlStream`, we
        // cannot be strict about what is going on in `start_send`:
        // `poll_ready` does not know what kind of data will be written (so
        // it could not make a size estimate, even if that was at all
        // possible with AsXml) and `start_send` is not a coroutine. So if
        // `Sink<T: AsXml>` wants to use `RawXmlStream`, it must be able to
        // submit an entire XSO's items in one batch to `RawXmlStream` after
        // it has reported to be ready once. That may easily make the buffer
        // reach its high water mark.
        //
        // So if we checked that condition in `start_send` (as opposed to
        // `poll_ready`), we would cause situations where submitting XSOs
        // failed randomly (with a panic or other errors) and would have to
        // be retried later.
        //
        // While failing with e.g. io::ErrorKind::WouldBlock is something
        // that could be investigated later, it would still require being
        // able to make an accurate estimate of the number of bytes needed to
        // serialise any given `AsXml`, because as pointed out earlier, once
        // we have started, there is no going back.
        //
        // Finally, none of that hurts much because `RawXmlStream` is only an
        // internal API. The high-level APIs will always call `poll_ready`
        // before sending an XSO, which means that we won't *grossly* go over
        // the TX buffer high water mark---unless you send a really large
        // XSO at once.
        tx_buffer_high_water_mark: usize,
    }
}

impl<Io: AsyncBufRead + AsyncWrite> RawXmlStream<Io> {
    fn new_writer(
        stream_ns: &'static str,
    ) -> rxml::writer::Encoder<rxml::writer::SimpleNamespaces> {
        let mut writer = rxml::writer::Encoder::new();
        writer
            .ns_tracker_mut()
            .declare_fixed(Some(xml_ncname!("stream")), XML_STREAM_NS.into());
        writer
            .ns_tracker_mut()
            .declare_fixed(None, stream_ns.into());
        writer
    }

    pub(super) fn new(io: Io, stream_ns: &'static str) -> Self {
        let parser = rxml::Parser::default();
        Self {
            parser: rxml::AsyncReader::wrap(io, parser),
            writer: Self::new_writer(stream_ns),
            stream_ns,
            tx_buffer: BytesMut::new(),

            // This basically means: "if we already have 2 kiB in our send
            // buffer, do not accept more data".
            // Please see the extensive words at
            //`Self::tx_buffer_high_water_mark` for details.
            tx_buffer_high_water_mark: 2048,
        }
    }

    pub(super) fn reset_state(self: Pin<&mut Self>) {
        let this = self.project();
        *this.parser.parser_pinned() = rxml::Parser::default();
        *this.writer = Self::new_writer(this.stream_ns);
    }
}

impl<Io> RawXmlStream<Io> {
    fn parser_pinned(self: Pin<&mut Self>) -> &mut rxml::Parser {
        self.project().parser.parser_pinned()
    }

    pub(super) fn get_stream(&self) -> &Io {
        self.parser.inner()
    }
}

impl<Io: AsyncBufRead> Stream for RawXmlStream<Io> {
    type Item = Result<rxml::Event, io::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        loop {
            return Poll::Ready(
                match ready!(this.parser.as_mut().poll_read(cx)).transpose() {
                    // Skip the XML declaration, nobody wants to hear about that.
                    Some(Ok(rxml::Event::XmlDeclaration(_, _))) => continue,
                    other => other,
                },
            );
        }
    }
}

impl<'x, Io: AsyncWrite> RawXmlStreamProj<'x, Io> {
    fn progress_write(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        while self.tx_buffer.len() > 0 {
            let written = match ready!(self
                .parser
                .as_mut()
                .inner_pinned()
                .poll_write(cx, &self.tx_buffer))
            {
                Ok(v) => v,
                Err(e) => return Poll::Ready(Err(e)),
            };
            self.tx_buffer.advance(written);
        }
        Poll::Ready(Ok(()))
    }
}

impl<'x, Io: AsyncWrite> Sink<xso::Item<'x>> for RawXmlStream<Io> {
    type Error = io::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut this = self.project();
        match this.progress_write(cx) {
            // No progress on write, but if we have enough space in the buffer
            // it's ok nonetheless.
            Poll::Pending => (),
            // Some progress and it went fine, move on.
            Poll::Ready(Ok(())) => (),
            // Something went wrong -> return the error.
            Poll::Ready(Err(e)) => return Poll::Ready(Err(e.into())),
        }
        if this.tx_buffer.len() < *this.tx_buffer_high_water_mark {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut this = self.project();
        ready!(this.progress_write(cx))?;
        this.parser.as_mut().inner_pinned().poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut this = self.project();
        ready!(this.progress_write(cx))?;
        this.parser.as_mut().inner_pinned().poll_shutdown(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: xso::Item<'x>) -> Result<(), Self::Error> {
        let this = self.project();
        this.writer
            .encode_into_bytes(item.as_rxml_item(), this.tx_buffer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
    }
}

/// Error returned by the [`ReadXso`] future and the [`ReadXsoState`] helper.
pub(super) enum ReadXsoError {
    /// The outer element was closed before a child element could be read.
    ///
    /// This is typically the stream footer in XML stream applications.
    Footer,

    /// A hard error occurred.
    ///
    /// This is either a real I/O error or an error from the XML parser.
    /// Neither are recoverable, because the nesting state is lost and
    /// in addition, XML errors are not recoverable because they indicate a
    /// not well-formed document.
    Hard(io::Error),

    /// A parse error occurred.
    ///
    /// The XML structure was well-formed, but the data contained did not
    /// match the XSO which was attempted to be parsed. This error is
    /// recoverable: when this error is emitted, the XML stream is at the same
    /// nesting level as it was before the XSO was attempted to be read; all
    /// XML structure which belonged to the XSO which failed to parse has
    /// been consumed. This allows to read more XSOs even if one fails to
    /// parse.
    Parse(xso::error::Error),
}

impl From<ReadXsoError> for io::Error {
    fn from(other: ReadXsoError) -> Self {
        match other {
            ReadXsoError::Hard(v) => v,
            ReadXsoError::Parse(e) => io::Error::new(io::ErrorKind::InvalidData, e),
            ReadXsoError::Footer => io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "element footer while waiting for XSO element start",
            ),
        }
    }
}

impl From<io::Error> for ReadXsoError {
    fn from(other: io::Error) -> Self {
        Self::Hard(other)
    }
}

impl From<xso::error::Error> for ReadXsoError {
    fn from(other: xso::error::Error) -> Self {
        Self::Parse(other)
    }
}

/// State for reading an XSO from a `Stream<Item = Result<rxml::Event, ...>>`.
///
/// Due to pinning, it is simpler to implement the statemachine in a dedicated
/// enum and let the actual (pinned) future pass the stream toward this enum's
/// function.
///
/// This is used by both [`ReadXso`] and the [`super::XmlStream`] itself.
#[derive(Default)]
pub(super) enum ReadXsoState<T: FromXml> {
    /// The [`rxml::Event::StartElement`] event was not seen yet.
    ///
    /// In this state, XML whitespace is ignored (as per RFC 6120 § 11.7), but
    /// other text data is rejected.
    #[default]
    PreData,

    /// The [`rxml::Event::StartElement`] event was received.
    ///
    /// The inner value is the builder for the "return type" of this enum and
    /// the implementation in the [`xso`] crate does all the heavy lifting:
    /// we'll only send events in its general direction.
    // We use the fallible parsing here so that we don't have to do the depth
    // accounting ourselves.
    Parsing(<Result<T, xso::error::Error> as FromXml>::Builder),

    /// The parsing has completed (successful or not).
    ///
    /// This is a final state and attempting to advance the state will panic.
    /// This is in accordance with [`core::future::Future::poll`]'s contract,
    /// for which this enum is primarily used.
    Done,
}

impl<T: FromXml> ReadXsoState<T> {
    /// Progress reading the XSO from the given source.
    ///
    /// This attempts to parse a single XSO from the underlying stream,
    /// while discarding any XML whitespace before the beginning of the XSO.
    ///
    /// If the XSO is parsed successfully, the method returns Ready with the
    /// parsed value. If parsing fails or an I/O error occurs, an appropriate
    /// error is returned.
    ///
    /// If parsing fails, the entire XML subtree belonging to the XSO is
    /// nonetheless processed. That makes parse errors recoverable: After
    /// `poll_advance` has returned Ready with either  an Ok result or a
    /// [`ReadXsoError::Parse`] error variant, another XSO can be read and the
    /// XML parsing will be at the same nesting depth as it was before the
    /// first call to `poll_advance`.
    ///
    /// Note that this guarantee does not hold for non-parse errors (i.e. for
    /// the other variants of [`ReadXsoError`]): I/O errors as well as
    /// occurrence of the outer closing element are fatal.
    ///
    /// The `source` passed to `poll_advance` should be the same on every
    /// call.
    pub(super) fn poll_advance<Io: AsyncBufRead>(
        &mut self,
        mut source: Pin<&mut RawXmlStream<Io>>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<T, ReadXsoError>> {
        loop {
            // Disable text buffering before the start event. That way, we
            // don't accumulate infinite amounts of XML whitespace caused by
            // whitespace keepalives.
            // (And also, we'll know faster when the remote side sends
            // non-whitespace garbage.)
            let text_buffering = match self {
                ReadXsoState::PreData => false,
                _ => true,
            };
            source
                .as_mut()
                .parser_pinned()
                .set_text_buffering(text_buffering);
            let ev = ready!(source.as_mut().poll_next(cx)).transpose()?;
            match self {
                ReadXsoState::PreData => match ev {
                    Some(rxml::Event::XmlDeclaration(_, _)) => (),
                    Some(rxml::Event::Text(_, data)) => {
                        if xso::is_xml_whitespace(data.as_bytes()) {
                            continue;
                        } else {
                            *self = ReadXsoState::Done;
                            return Poll::Ready(Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "non-whitespace text content before XSO",
                            )
                            .into()));
                        }
                    }
                    Some(rxml::Event::StartElement(_, name, attrs)) => {
                        *self = ReadXsoState::Parsing(
                            <Result<T, xso::error::Error> as FromXml>::from_events(name, attrs)
                                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                        );
                    }
                    // Amounts to EOF, as we expect to start on the stream level.
                    Some(rxml::Event::EndElement(_)) => {
                        *self = ReadXsoState::Done;
                        return Poll::Ready(Err(ReadXsoError::Footer));
                    }
                    None => {
                        *self = ReadXsoState::Done;
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "end of parent element before XSO started",
                        )
                        .into()));
                    }
                },
                ReadXsoState::Parsing(builder) => {
                    let Some(ev) = ev else {
                        *self = ReadXsoState::Done;
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "eof during XSO parsing",
                        )
                        .into()));
                    };

                    match builder.feed(ev) {
                        Err(err) => {
                            *self = ReadXsoState::Done;
                            return Poll::Ready(Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                err,
                            )
                            .into()));
                        }
                        Ok(Some(Err(err))) => {
                            *self = ReadXsoState::Done;
                            return Poll::Ready(Err(ReadXsoError::Parse(err)));
                        }
                        Ok(Some(Ok(value))) => {
                            *self = ReadXsoState::Done;
                            return Poll::Ready(Ok(value));
                        }
                        Ok(None) => (),
                    }
                }

                // The error talks about "future", simply because that is
                // where `Self` is used (inside `core::future::Future::poll`).
                ReadXsoState::Done => panic!("future polled after completion"),
            }
        }
    }
}

/// Future to read a single XSO from a stream.
pub(super) struct ReadXso<'x, Io, T: FromXml> {
    /// Stream to read the future from.
    inner: Pin<&'x mut RawXmlStream<Io>>,

    /// Current state of parsing.
    state: ReadXsoState<T>,
}

impl<'x, Io: AsyncBufRead, T: FromXml> ReadXso<'x, Io, T> {
    /// Start reading a single XSO from a stream.
    pub(super) fn read_from(stream: Pin<&'x mut RawXmlStream<Io>>) -> Self {
        Self {
            inner: stream,
            state: ReadXsoState::PreData,
        }
    }
}

impl<'x, Io: AsyncBufRead, T: FromXml> Future for ReadXso<'x, Io, T>
where
    T::Builder: Unpin,
{
    type Output = Result<T, ReadXsoError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        this.state.poll_advance(this.inner.as_mut(), cx)
    }
}

/// Contains metadata from an XML stream header
#[derive(Default)]
pub struct StreamHeader<'x> {
    /// The optional `from` attribute.
    pub from: Option<Cow<'x, str>>,

    /// The optional `to` attribute.
    pub to: Option<Cow<'x, str>>,

    /// The optional `id` attribute.
    pub id: Option<Cow<'x, str>>,
}

impl<'x> StreamHeader<'x> {
    /// Take the contents and return them as new object.
    ///
    /// `self` will be left with all its parts set to `None`.
    pub fn take(&mut self) -> Self {
        Self {
            from: self.from.take(),
            to: self.to.take(),
            id: self.id.take(),
        }
    }

    pub(super) async fn send<Io: AsyncWrite>(
        self,
        mut stream: Pin<&mut RawXmlStream<Io>>,
    ) -> io::Result<()> {
        stream
            .send(Item::XmlDeclaration(rxml::XmlVersion::V1_0))
            .await?;
        stream
            .send(Item::ElementHeadStart(
                Namespace::from(XML_STREAM_NS),
                Cow::Borrowed(xml_ncname!("stream")),
            ))
            .await?;
        if let Some(from) = self.from {
            stream
                .send(Item::Attribute(
                    Namespace::NONE,
                    Cow::Borrowed(xml_ncname!("from")),
                    from,
                ))
                .await?;
        }
        if let Some(to) = self.to {
            stream
                .send(Item::Attribute(
                    Namespace::NONE,
                    Cow::Borrowed(xml_ncname!("to")),
                    to,
                ))
                .await?;
        }
        if let Some(id) = self.id {
            stream
                .send(Item::Attribute(
                    Namespace::NONE,
                    Cow::Borrowed(xml_ncname!("id")),
                    id,
                ))
                .await?;
        }
        stream
            .send(Item::Attribute(
                Namespace::NONE,
                Cow::Borrowed(xml_ncname!("version")),
                Cow::Borrowed("1.0"),
            ))
            .await?;
        stream.send(Item::ElementHeadEnd).await?;
        Ok(())
    }
}

impl StreamHeader<'static> {
    pub(super) async fn recv<Io: AsyncBufRead>(
        mut stream: Pin<&mut RawXmlStream<Io>>,
    ) -> io::Result<Self> {
        loop {
            match stream.as_mut().next().await.transpose()? {
                Some(Event::StartElement(_, (ns, name), mut attrs)) => {
                    if ns != XML_STREAM_NS || name != "stream" {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "unknown stream header",
                        ));
                    }

                    match attrs.remove(Namespace::none(), "version") {
                        Some(v) => {
                            if v != "1.0" {
                                return Err(io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    format!("unsuppored stream version: {}", v),
                                ));
                            }
                        }
                        None => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "required `version` attribute missing",
                            ))
                        }
                    }

                    let from = attrs.remove(Namespace::none(), "from");
                    let to = attrs.remove(Namespace::none(), "to");
                    let id = attrs.remove(Namespace::none(), "id");
                    let _ = attrs.remove(Namespace::xml(), "lang");

                    if let Some(((ns, name), _)) = attrs.into_iter().next() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("unexpected stream header attribute: {{{}}}{}", ns, name),
                        ));
                    }

                    return Ok(StreamHeader {
                        from: from.map(Cow::Owned),
                        to: to.map(Cow::Owned),
                        id: id.map(Cow::Owned),
                    });
                }
                Some(Event::Text(_, _)) | Some(Event::EndElement(_)) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "unexpected content before stream header",
                    ))
                }
                // We cannot loop infinitely here because the XML parser will
                // prevent more than one XML declaration from being parsed.
                Some(Event::XmlDeclaration(_, _)) => (),
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "eof before stream header",
                    ))
                }
            }
        }
    }
}
