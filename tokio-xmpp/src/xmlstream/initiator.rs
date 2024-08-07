// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use core::pin::Pin;
use std::borrow::Cow;
use std::io;

use tokio::io::{AsyncBufRead, AsyncWrite};

use xmpp_parsers::stream_features::StreamFeatures;

use xso::{AsXml, FromXml};

use super::{
    common::{RawXmlStream, ReadXso, StreamHeader},
    XmlStream,
};

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
}
