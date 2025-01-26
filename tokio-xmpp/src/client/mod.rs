// Copyright (c) 2019 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io;

use xmpp_parsers::{jid::Jid, stream_features::StreamFeatures};

use crate::{
    connect::ServerConnector,
    error::Error,
    stanzastream::{StanzaStage, StanzaState, StanzaStream, StanzaToken},
    xmlstream::Timeouts,
    Stanza,
};

#[cfg(any(feature = "starttls", feature = "insecure-tcp"))]
use crate::connect::DnsConfig;
#[cfg(feature = "starttls")]
use crate::connect::StartTlsServerConnector;
#[cfg(feature = "insecure-tcp")]
use crate::connect::TcpServerConnector;

pub(crate) mod login;
mod stream;

/// XMPP client connection and state
///
/// This implements the `futures` crate's [`Stream`](#impl-Stream) to receive
/// stream state changes as well as stanzas received via the stream.
///
/// To send stanzas, the [`send_stanza`][`Client::send_stanza`] method can be
/// used.
pub struct Client {
    stream: StanzaStream,
    bound_jid: Option<Jid>,
    features: Option<StreamFeatures>,
}

impl Client {
    /// Get the client's bound JID (the one reported by the XMPP
    /// server).
    pub fn bound_jid(&self) -> Option<&Jid> {
        self.bound_jid.as_ref()
    }

    /// Send a stanza.
    ///
    /// This will automatically allocate an ID if the stanza has no ID set.
    /// The returned `StanzaToken` is awaited up to the [`StanzaStage::Sent`]
    /// stage, which means that this coroutine only returns once the stanza
    /// has actually been written to the XMPP transport.
    ///
    /// Note that this does not imply that it has been *reeceived* by the
    /// peer, nor that it has been successfully processed. To confirm that a
    /// stanza has been received by a peer, the [`StanzaToken::wait_for`]
    /// method can be called with [`StanzaStage::Acked`], but that stage will
    /// only ever be reached if the server supports XEP-0198 and it has been
    /// negotiated successfully (this may change in the future).
    pub async fn send_stanza(&mut self, mut stanza: Stanza) -> Result<StanzaToken, io::Error> {
        stanza.ensure_id();
        let mut token = self.stream.send(Box::new(stanza)).await;
        match token.wait_for(StanzaStage::Sent).await {
            // Queued < Sent, so it cannot be reached.
            Some(StanzaState::Queued) => unreachable!(),

            None | Some(StanzaState::Dropped) => Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "stream disconnected fatally before stanza could be sent",
            )),
            Some(StanzaState::Failed { error }) => Err(error.into_io_error()),
            Some(StanzaState::Sent { .. }) | Some(StanzaState::Acked { .. }) => Ok(token),
        }
    }

    /// Get the stream features (`<stream:features/>`) of the underlying
    /// stream.
    ///
    /// If the stream has not completed negotiation yet, this will return
    /// `None`. Note that stream features may change at any point due to a
    /// transparent reconnect.
    pub fn get_stream_features(&self) -> Option<&StreamFeatures> {
        self.features.as_ref()
    }

    /// Close the client cleanly.
    ///
    /// This performs an orderly stream shutdown, ensuring that all resources
    /// are correctly cleaned up.
    pub async fn send_end(self) -> Result<(), Error> {
        self.stream.close().await;
        Ok(())
    }
}

#[cfg(feature = "starttls")]
impl Client {
    /// Start a new XMPP client using StartTLS transport and autoreconnect
    ///
    /// Start polling the returned instance so that it will connect
    /// and yield events.
    pub fn new<J: Into<Jid>, P: Into<String>>(jid: J, password: P) -> Self {
        let jid = jid.into();
        let dns_config = DnsConfig::srv(&jid.domain().to_string(), "_xmpp-client._tcp", 5222);
        Self::new_starttls(jid, password, dns_config, Timeouts::default())
    }

    /// Start a new XMPP client with StartTLS transport and specific DNS config
    pub fn new_starttls<J: Into<Jid>, P: Into<String>>(
        jid: J,
        password: P,
        dns_config: DnsConfig,
        timeouts: Timeouts,
    ) -> Self {
        Self::new_with_connector(
            jid,
            password,
            StartTlsServerConnector::from(dns_config),
            timeouts,
        )
    }
}

#[cfg(feature = "insecure-tcp")]
impl Client {
    /// Start a new XMPP client with plaintext insecure connection and specific DNS config
    pub fn new_plaintext<J: Into<Jid>, P: Into<String>>(
        jid: J,
        password: P,
        dns_config: DnsConfig,
        timeouts: Timeouts,
    ) -> Self {
        Self::new_with_connector(
            jid,
            password,
            TcpServerConnector::from(dns_config),
            timeouts,
        )
    }
}

impl Client {
    /// Start a new client given that the JID is already parsed.
    pub fn new_with_connector<J: Into<Jid>, P: Into<String>, C: ServerConnector>(
        jid: J,
        password: P,
        connector: C,
        timeouts: Timeouts,
    ) -> Self {
        Self {
            stream: StanzaStream::new_c2s(connector, jid.into(), password.into(), timeouts, 16),
            bound_jid: None,
            features: None,
        }
    }
}
