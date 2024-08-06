//! `starttls::ServerConfig` provides a `ServerConnector` for starttls connections

use tokio::net::TcpStream;

use crate::{
    connect::{DnsConfig, ServerConnector},
    proto::XmppStream,
    Client, Component, Error,
};

/// Component that connects over TCP
pub type TcpComponent = Component<TcpServerConnector>;

/// Client that connects over TCP
pub type TcpClient = Client<TcpServerConnector>;

/// Connect via insecure plaintext TCP to an XMPP server
/// This should only be used over localhost or otherwise when you know what you are doing
/// Probably mostly useful for Components
#[derive(Debug, Clone)]
pub struct TcpServerConnector(pub DnsConfig);

impl From<DnsConfig> for TcpServerConnector {
    fn from(dns_config: DnsConfig) -> TcpServerConnector {
        Self(dns_config)
    }
}

impl ServerConnector for TcpServerConnector {
    type Stream = TcpStream;

    async fn connect(
        &self,
        jid: &xmpp_parsers::jid::Jid,
        ns: &str,
    ) -> Result<XmppStream<Self::Stream>, Error> {
        let stream = self.0.resolve().await?;
        Ok(XmppStream::start(stream, jid.clone(), ns.to_owned()).await?)
    }
}
