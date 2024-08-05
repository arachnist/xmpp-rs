//! `starttls::ServerConfig` provides a `ServerConnector` for starttls connections

use std::sync::Arc;

use tokio::net::TcpStream;

use crate::{connect::ServerConnector, xmpp_stream::XMPPStream, Component, Error};

/// Component that connects over TCP
pub type TcpComponent = Component<TcpServerConnector>;

/// Connect via insecure plaintext TCP to an XMPP server
/// This should only be used over localhost or otherwise when you know what you are doing
/// Probably mostly useful for Components
#[derive(Debug, Clone)]
pub struct TcpServerConnector(Arc<String>);

impl TcpServerConnector {
    /// Create a new connector with the given address
    pub fn new(addr: String) -> Self {
        Self(addr.into())
    }
}

impl ServerConnector for TcpServerConnector {
    type Stream = TcpStream;
    async fn connect(
        &self,
        jid: &xmpp_parsers::jid::Jid,
        ns: &str,
    ) -> Result<XMPPStream<Self::Stream>, Error> {
        let stream = TcpStream::connect(&*self.0)
            .await
            .map_err(|e| crate::Error::Io(e))?;
        Ok(XMPPStream::start(stream, jid.clone(), ns.to_owned()).await?)
    }
}

impl Component<TcpServerConnector> {
    /// Start a new XMPP component
    pub async fn new(jid: &str, password: &str, server: String) -> Result<Self, Error> {
        Self::new_with_connector(jid, password, TcpServerConnector::new(server)).await
    }
}

