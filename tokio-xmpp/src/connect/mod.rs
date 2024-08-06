//! `ServerConnector` provides streams for XMPP clients

use sasl::common::ChannelBinding;
use tokio::io::{AsyncRead, AsyncWrite};
use xmpp_parsers::jid::Jid;

use crate::proto::XmppStream;
use crate::Error;

#[cfg(feature = "starttls")]
pub mod starttls;
#[cfg(feature = "starttls")]
pub use starttls::StartTlsServerConnector;

#[cfg(feature = "insecure-tcp")]
pub mod tcp;
#[cfg(feature = "insecure-tcp")]
pub use tcp::TcpServerConnector;

mod dns;
pub use dns::DnsConfig;

/// trait returned wrapped in XmppStream by ServerConnector
pub trait AsyncReadAndWrite: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> AsyncReadAndWrite for T {}

/// Trait that must be extended by the implementation of ServerConnector
pub trait ServerConnectorError: std::error::Error + Sync + Send {}

/// Trait called to connect to an XMPP server, perhaps called multiple times
pub trait ServerConnector: Clone + core::fmt::Debug + Send + Unpin + 'static {
    /// The type of Stream this ServerConnector produces
    type Stream: AsyncReadAndWrite;
    /// This must return the connection ready to login, ie if starttls is involved, after TLS has been started, and then after the <stream headers are exchanged
    fn connect(
        &self,
        jid: &Jid,
        ns: &str,
    ) -> impl std::future::Future<Output = Result<XmppStream<Self::Stream>, Error>> + Send;

    /// Return channel binding data if available
    /// do not fail if channel binding is simply unavailable, just return Ok(None)
    /// this should only be called after the TLS handshake is finished
    fn channel_binding(_stream: &Self::Stream) -> Result<ChannelBinding, Error> {
        Ok(ChannelBinding::None)
    }
}
