// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use core::pin::Pin;
use std::borrow::Cow;
use std::io;

use futures::SinkExt;

use tokio::io::{AsyncBufRead, AsyncWrite};

use xmpp_parsers::stream_features::StreamFeatures;

use xso::{AsXml, FromXml};

use super::{
    common::{RawXmlStream, ReadXso, StreamHeader},
    XmlStream,
};

/// Type state for an initiator stream which has not yet sent its stream
/// header.
///
/// To continue stream setup, call [`send_header`][`Self::send_header`].
pub struct InitiatingStream<Io>(pub(super) RawXmlStream<Io>);

impl<Io: AsyncBufRead + AsyncWrite + Unpin> InitiatingStream<Io> {
    /// Send the stream header.
    pub async fn send_header(
        self,
        header: StreamHeader<'_>,
    ) -> io::Result<PendingFeaturesRecv<Io>> {
        let Self(mut stream) = self;

        header.send(Pin::new(&mut stream)).await?;
        stream.flush().await?;
        let header = StreamHeader::recv(Pin::new(&mut stream)).await?;
        Ok(PendingFeaturesRecv { stream, header })
    }
}

/// Type state for an initiator stream which has sent and received the stream
/// header.
///
/// To continue stream setup, call [`recv_features`][`Self::recv_features`].
pub struct PendingFeaturesRecv<Io> {
    pub(super) stream: RawXmlStream<Io>,
    pub(super) header: StreamHeader<'static>,
}

impl<Io> PendingFeaturesRecv<Io> {
    /// The stream header contents as sent by the peer.
    pub fn header(&self) -> StreamHeader<'_> {
        StreamHeader {
            from: self.header.from.as_ref().map(|x| Cow::Borrowed(&**x)),
            to: self.header.to.as_ref().map(|x| Cow::Borrowed(&**x)),
            id: self.header.id.as_ref().map(|x| Cow::Borrowed(&**x)),
        }
    }

    /// Extract the stream header contents as sent by the peer.
    pub fn take_header(&mut self) -> StreamHeader<'static> {
        self.header.take()
    }
}

impl<Io: AsyncBufRead + AsyncWrite + Unpin> PendingFeaturesRecv<Io> {
    /// Receive the responder's stream features.
    ///
    /// After the stream features have been received, the stream can be used
    /// for exchanging stream-level elements (stanzas or "nonzas"). The Rust
    /// type for these elements must be given as type parameter `T`.
    pub async fn recv_features<T: FromXml + AsXml>(
        self,
    ) -> io::Result<(StreamFeatures, XmlStream<Io, T>)> {
        let Self {
            mut stream,
            header: _,
        } = self;
        let features = ReadXso::read_from(Pin::new(&mut stream)).await?;
        Ok((features, XmlStream::wrap(stream)))
    }

    /// Skip receiving the responder's stream features.
    ///
    /// The stream can be used for exchanging stream-level elements (stanzas
    /// or "nonzas"). The Rust type for these elements must be given as type
    /// parameter `T`.
    ///
    /// **Note:** Using this on RFC 6120 compliant streams where stream
    /// features **are** sent after the stream header will cause a parse error
    /// down the road (because the feature stream element cannot be handled).
    /// The only place where this is useful is in
    /// [XEP-0114](https://xmpp.org/extensions/xep-0114.html) connections.
    pub fn skip_features<T: FromXml + AsXml>(self) -> XmlStream<Io, T> {
        XmlStream::wrap(self.stream)
    }
}
