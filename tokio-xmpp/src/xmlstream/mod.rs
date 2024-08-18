// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! # RFC 6120 XML Streams
//!
//! **Note:** The XML stream is a low-level API which you should probably not
//! use directly.
//!
//! Establishing an XML stream is always a multi-stage process due to how
//! stream negotiation works. Based on the values sent by the initiator in the
//! stream header, the responder may choose to offer different features.
//!
//! In order to allow this, the following multi-step processes are defined.
//!
//! ## Initiating an XML stream
//!
//! To initiate an XML stream, you need to:
//!
//! 1. Call [`initiate_stream`] to obtain the [`PendingFeaturesRecv`] object.
//!    That object holds the stream header sent by the peer for inspection.
//! 2. Call [`PendingFeaturesRecv::recv_features`] if you are content with
//!    the content of the stream header to obtain the [`XmlStream`] object and
//!    the features sent by the peer.
//!
//! ## Accepting an XML stream connection
//!
//! To accept an XML stream, you need to:
//!
//! 1. Call [`accept_stream`] to obtain the [`AcceptedStream`] object.
//!    That object holds the stream header sent by the peer for inspection.
//! 2. Call [`AcceptedStream::send_header`] if you are content with
//!    the content of the stream header to obtain the [`PendingFeaturesSend`]
//!    object.
//! 3. Call [`PendingFeaturesSend::send_features`] to send the stream features
//!    to the peer and obtain the [`XmlStream`] object.
//!
//! ## Mid-stream resets
//!
//! RFC 6120 describes a couple of situations where stream resets are executed
//! during stream negotiation. During a stream reset, both parties drop their
//! parser state and the stream is started from the beginning, with a new
//! stream header sent by the initiator and received by the responder.
//!
//! Stream resets are inherently prone to race conditions. If the responder
//! executes a read from the underlying transport between sending the element
//! which triggers the stream reset and discarding its parser state, it may
//! accidentally read the initiator's stream header into the *old* parser
//! state instead of the post-reset parser state.
//!
//! Stream resets are executed with the [`XmlStream::initiate_reset`] and
//! [`XmlStream::accept_reset`] functions, for initiator and responder,
//! respectively. In order to avoid the race condition,
//! [`XmlStream::accept_reset`] handles sending the last pre-reset element and
//! resetting the stream in a single step.

use core::fmt;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::io;
#[cfg(feature = "syntax-highlighting")]
use std::sync::OnceLock;

use futures::{ready, Sink, SinkExt, Stream};

use tokio::io::{AsyncBufRead, AsyncWrite};

use xso::{AsXml, FromXml, Item};

mod capture;
mod common;
mod initiator;
mod responder;
#[cfg(test)]
mod tests;
pub(crate) mod xmpp;

pub use self::common::StreamHeader;
use self::common::{RawXmlStream, ReadXsoError, ReadXsoState};
pub use self::initiator::{InitiatingStream, PendingFeaturesRecv};
pub use self::responder::{AcceptedStream, PendingFeaturesSend};
pub use self::xmpp::XmppStreamElement;

#[cfg(feature = "syntax-highlighting")]
static PS: OnceLock<syntect::parsing::SyntaxSet> = OnceLock::new();
#[cfg(feature = "syntax-highlighting")]
static SYNTAX: OnceLock<syntect::parsing::SyntaxReference> = OnceLock::new();
#[cfg(feature = "syntax-highlighting")]
static THEME: OnceLock<syntect::highlighting::Theme> = OnceLock::new();

#[cfg(feature = "syntax-highlighting")]
fn highlight_xml(xml: &str) -> String {
    let ps = PS.get_or_init(syntect::parsing::SyntaxSet::load_defaults_newlines);
    let mut h = syntect::easy::HighlightLines::new(
        SYNTAX.get_or_init(|| ps.find_syntax_by_extension("xml").unwrap().clone()),
        THEME.get_or_init(|| {
            syntect::highlighting::ThemeSet::load_defaults().themes["Solarized (dark)"].clone()
        }),
    );

    let ranges: Vec<_> = h.highlight_line(&xml, ps).unwrap();
    let escaped = syntect::util::as_24_bit_terminal_escaped(&ranges[..], false);
    format!("{}\x1b[0m", escaped)
}

struct LogXsoBuf<'x>(&'x [u8]);

impl<'x> fmt::Display for LogXsoBuf<'x> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // We always generate UTF-8, so this should be good... I think.
        let text = std::str::from_utf8(&self.0).unwrap();
        #[cfg(feature = "syntax-highlighting")]
        let text = highlight_xml(text);
        f.write_str(&text)
    }
}

/// Initiate a new stream
///
/// Initiate a new stream using the given I/O object `io`. The default
/// XML namespace will be set to `stream_ns` and the stream header will use
/// the attributes as set in `stream_header`, along with version `1.0`.
///
/// The returned object contains the stream header sent by the remote side
/// as well as the internal parser state to continue the negotiation.
pub async fn initiate_stream<Io: AsyncBufRead + AsyncWrite + Unpin>(
    io: Io,
    stream_ns: &'static str,
    stream_header: StreamHeader<'_>,
) -> Result<PendingFeaturesRecv<Io>, io::Error> {
    let stream = InitiatingStream(RawXmlStream::new(io, stream_ns));
    stream.send_header(stream_header).await
}

/// Accept a new XML stream as responder
///
/// Prepares the responer side of an XML stream using the given I/O object
/// `io`. The default XML namespace will be set to `stream_ns`.
///
/// The returned object contains the stream header sent by the remote side
/// as well as the internal parser state to continue the negotiation.
pub async fn accept_stream<Io: AsyncBufRead + AsyncWrite + Unpin>(
    io: Io,
    stream_ns: &'static str,
) -> Result<AcceptedStream<Io>, io::Error> {
    let mut stream = RawXmlStream::new(io, stream_ns);
    let header = StreamHeader::recv(Pin::new(&mut stream)).await?;
    Ok(AcceptedStream { stream, header })
}

/// A non-success state which may occur while reading an XSO from a
/// [`XmlStream`]
#[derive(Debug)]
pub enum ReadError {
    /// The soft timeout of the stream triggered.
    ///
    /// User code should handle this by sending something into the stream
    /// which causes the peer to send data before the hard timeout triggers.
    SoftTimeout,

    /// An I/O error occurred in the underlying I/O object.
    ///
    /// This is generally fatal.
    HardError(io::Error),

    /// A parse error occurred while processing the XSO.
    ///
    /// This is non-fatal and more XSOs may be read from the stream.
    ParseError(xso::error::Error),

    /// The stream footer was received.
    ///
    /// Any future read attempts will again return this error. The stream has
    /// been closed by the peer and you should probably close it, too.
    StreamFooterReceived,
}

enum WriteState {
    Open,
    SendElementFoot,
    FooterSent,
    Failed,
}

impl WriteState {
    fn check_ok(&self) -> io::Result<()> {
        match self {
            WriteState::Failed => Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "XML stream sink unusable because of previous write error",
            )),
            WriteState::Open | WriteState::SendElementFoot | WriteState::FooterSent => Ok(()),
        }
    }

    fn check_writable(&self) -> io::Result<()> {
        match self {
            WriteState::SendElementFoot | WriteState::FooterSent => Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "stream footer already sent",
            )),
            WriteState::Failed | WriteState::Open => self.check_ok(),
        }
    }
}

pin_project_lite::pin_project! {
    /// XML stream
    ///
    /// This struct represents an
    /// [RFC 6120](https://tools.ietf.org/html/rfc6120) XML stream, where the
    /// payload consists of items of type `T` implementing [`FromXml`] and
    /// [`AsXml`].
    pub struct XmlStream<Io, T: FromXml> {
        #[pin]
        inner: RawXmlStream<Io>,
        read_state: Option<ReadXsoState<T>>,
        write_state: WriteState,
    }
}

impl<Io, T: FromXml> XmlStream<Io, T> {
    /// Obtain a reference to the `Io` stream.
    pub fn get_stream(&self) -> &Io {
        self.inner.get_stream()
    }
}

impl<Io: AsyncBufRead, T: FromXml + AsXml> XmlStream<Io, T> {
    fn wrap(inner: RawXmlStream<Io>) -> Self {
        Self {
            inner,
            read_state: Some(ReadXsoState::default()),
            write_state: WriteState::Open,
        }
    }

    fn assert_retypable(&self) {
        match self.read_state {
            Some(ReadXsoState::PreData) => (),
            Some(_) => panic!("cannot reset stream: XSO parsing in progress!"),
            None => panic!("cannot reset stream: stream footer received!"),
        }
        match self.write_state.check_writable() {
            Ok(()) => (),
            Err(e) => panic!("cannot reset stream: {}", e),
        }
    }
}

impl<Io: AsyncBufRead + AsyncWrite + Unpin, T: FromXml + AsXml + fmt::Debug> XmlStream<Io, T> {
    /// Initiate a stream reset
    ///
    /// To actually send the stream header, call
    /// [`send_header`][`InitiatingStream::send_header`] on the result.
    ///
    /// # Panics
    ///
    /// Attempting to reset the stream while an object is being received will
    /// panic. This can generally only happen if you call `poll_next`
    /// directly, as doing that is otherwise prevented by the borrowchecker.
    ///
    /// In addition, attempting to reset a stream which has been closed by
    /// either side or which has had an I/O error will also cause a panic.
    pub fn initiate_reset(self) -> InitiatingStream<Io> {
        self.assert_retypable();

        let mut stream = self.inner;
        Pin::new(&mut stream).reset_state();
        InitiatingStream(stream)
    }

    /// Trigger a stream reset on the initiator side and await the new stream
    /// header.
    ///
    /// This is the responder-side counterpart to
    /// [`initiate_reset`][`Self::initiate_reset`]. The element which causes
    /// the stream reset must be passed as `barrier` and it will be sent
    /// right before resetting the parser state. This way, the race condition
    /// outlined in the [`xmlstream`][`self`] module's documentation is
    /// guaranteed to be avoided.
    ///
    /// Note that you should not send the element passed as `barrier` down the
    /// stream yourself, as this function takes care of it.
    ///
    /// # Stream resets without a triggering element
    ///
    /// These are not possible to do safely and not specified in RFC 6120,
    /// hence they cannot be done in [`XmlStream`].
    ///
    /// # Panics
    ///
    /// Attempting to reset the stream while an object is being received will
    /// panic. This can generally only happen if you call `poll_next`
    /// directly, as doing that is otherwise prevented by the borrowchecker.
    ///
    /// In addition, attempting to reset a stream which has been closed by
    /// either side or which has had an I/O error will also cause a panic.
    pub async fn accept_reset(mut self, barrier: &T) -> io::Result<AcceptedStream<Io>> {
        self.assert_retypable();
        self.send(barrier).await?;

        let mut stream = self.inner;
        Pin::new(&mut stream).reset_state();
        let header = StreamHeader::recv(Pin::new(&mut stream)).await?;
        Ok(AcceptedStream { stream, header })
    }

    /// Discard all XML state and return the inner I/O object.
    pub fn into_inner(self) -> Io {
        self.assert_retypable();
        self.inner.into_inner()
    }
}

impl<Io: AsyncBufRead, T: FromXml + AsXml + fmt::Debug> Stream for XmlStream<Io, T> {
    type Item = Result<T, ReadError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let result = match this.read_state.as_mut() {
            None => return Poll::Ready(Some(Err(ReadError::StreamFooterReceived))),
            Some(read_state) => ready!(read_state.poll_advance(this.inner, cx)),
        };
        let result = match result {
            Ok(v) => Poll::Ready(Some(Ok(v))),
            Err(ReadXsoError::Hard(e)) => Poll::Ready(Some(Err(ReadError::HardError(e)))),
            Err(ReadXsoError::Parse(e)) => Poll::Ready(Some(Err(ReadError::ParseError(e)))),
            Err(ReadXsoError::Footer) => {
                *this.read_state = None;
                Poll::Ready(Some(Err(ReadError::StreamFooterReceived)))
            }
        };
        *this.read_state = Some(ReadXsoState::default());
        result
    }
}

impl<'x, Io: AsyncWrite, T: FromXml + AsXml + fmt::Debug> Sink<&'x T> for XmlStream<Io, T> {
    type Error = io::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.write_state.check_writable()?;
        this.inner.poll_ready(cx)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.write_state.check_writable()?;
        this.inner.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut this = self.project();
        this.write_state.check_ok()?;
        loop {
            match this.write_state {
                // Open => initiate closing.
                WriteState::Open => {
                    *this.write_state = WriteState::SendElementFoot;
                }
                // Sending => wait for readiness, then send.
                WriteState::SendElementFoot => {
                    match ready!(this.inner.as_mut().poll_ready(cx))
                        .and_then(|_| this.inner.as_mut().start_send(Item::ElementFoot))
                    {
                        Ok(()) => (),
                        // If it fails, we fail the sink immediately.
                        Err(e) => {
                            *this.write_state = WriteState::Failed;
                            return Poll::Ready(Err(e));
                        }
                    }
                    *this.write_state = WriteState::FooterSent;
                }
                // Footer sent => just poll the inner sink for closure.
                WriteState::FooterSent => break,
                WriteState::Failed => unreachable!(), // caught by check_ok()
            }
        }
        this.inner.poll_close(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: &'x T) -> Result<(), Self::Error> {
        let this = self.project();
        this.write_state.check_writable()?;
        this.inner.start_send_xso(item)
    }
}

/// Convenience alias for an XML stream using [`XmppStreamElement`].
pub type XmppStream<Io> = XmlStream<Io, XmppStreamElement>;
