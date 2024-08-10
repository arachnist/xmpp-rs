use futures::sink::SinkExt;
use xmpp_parsers::{jid::Jid, stream_features::StreamFeatures};

use crate::{
    client::{login::client_login, stream::ClientState},
    connect::ServerConnector,
    error::Error,
    Stanza,
};

#[cfg(any(feature = "starttls", feature = "insecure-tcp"))]
use crate::connect::DnsConfig;
#[cfg(feature = "starttls")]
use crate::connect::StartTlsServerConnector;
#[cfg(feature = "insecure-tcp")]
use crate::connect::TcpServerConnector;

mod bind;
mod login;
mod stream;

/// XMPP client connection and state
///
/// It is able to reconnect. TODO: implement session management.
///
/// This implements the `futures` crate's [`Stream`](#impl-Stream) and
/// [`Sink`](#impl-Sink<Packet>) traits.
pub struct Client<C: ServerConnector> {
    jid: Jid,
    password: String,
    connector: C,
    state: ClientState<C::Stream>,
    reconnect: bool,
    // TODO: tls_required=true
}

impl<C: ServerConnector> Client<C> {
    /// Set whether to reconnect (`true`) or let the stream end
    /// (`false`) when a connection to the server has ended.
    pub fn set_reconnect(&mut self, reconnect: bool) -> &mut Self {
        self.reconnect = reconnect;
        self
    }

    /// Get the client's bound JID (the one reported by the XMPP
    /// server).
    pub fn bound_jid(&self) -> Option<&Jid> {
        match self.state {
            ClientState::Connected { ref bound_jid, .. } => Some(bound_jid),
            _ => None,
        }
    }

    /// Send stanza
    pub async fn send_stanza(&mut self, mut stanza: Stanza) -> Result<(), Error> {
        stanza.ensure_id();
        self.send(stanza).await
    }

    /// Get the stream features (`<stream:features/>`) of the underlying stream
    pub fn get_stream_features(&self) -> Option<&StreamFeatures> {
        match self.state {
            ClientState::Connected { ref features, .. } => Some(features),
            _ => None,
        }
    }

    /// End connection by sending `</stream:stream>`
    ///
    /// You may expect the server to respond with the same. This
    /// client will then drop its connection.
    ///
    /// Make sure to disable reconnect.
    pub async fn send_end(&mut self) -> Result<(), Error> {
        match self.state {
            ClientState::Connected { ref mut stream, .. } => Ok(stream.close().await?),
            ClientState::Connecting { .. } => {
                self.state = ClientState::Disconnected;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

#[cfg(feature = "starttls")]
impl Client<StartTlsServerConnector> {
    /// Start a new XMPP client using StartTLS transport and autoreconnect
    ///
    /// Start polling the returned instance so that it will connect
    /// and yield events.
    pub fn new<J: Into<Jid>, P: Into<String>>(jid: J, password: P) -> Self {
        let jid = jid.into();
        let mut client = Self::new_starttls(
            jid.clone(),
            password,
            DnsConfig::srv(&jid.domain().to_string(), "_xmpp-client._tcp", 5222),
        );
        client.set_reconnect(true);
        client
    }

    /// Start a new XMPP client with StartTLS transport and specific DNS config
    pub fn new_starttls<J: Into<Jid>, P: Into<String>>(
        jid: J,
        password: P,
        dns_config: DnsConfig,
    ) -> Self {
        Self::new_with_connector(jid, password, StartTlsServerConnector::from(dns_config))
    }
}

#[cfg(feature = "insecure-tcp")]
impl Client<TcpServerConnector> {
    /// Start a new XMPP client with plaintext insecure connection and specific DNS config
    pub fn new_plaintext<J: Into<Jid>, P: Into<String>>(
        jid: J,
        password: P,
        dns_config: DnsConfig,
    ) -> Self {
        Self::new_with_connector(jid, password, TcpServerConnector::from(dns_config))
    }
}

impl<C: ServerConnector> Client<C> {
    /// Start a new client given that the JID is already parsed.
    pub fn new_with_connector<J: Into<Jid>, P: Into<String>>(
        jid: J,
        password: P,
        connector: C,
    ) -> Self {
        let jid = jid.into();
        let password = password.into();

        let connect = tokio::spawn(client_login(
            connector.clone(),
            jid.clone(),
            password.clone(),
        ));
        let client = Client {
            jid,
            password,
            connector,
            state: ClientState::Connecting(connect),
            reconnect: false,
        };
        client
    }
}
