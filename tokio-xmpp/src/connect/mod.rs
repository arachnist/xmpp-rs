//! `ServerConnector` provides streams for XMPP clients

#[cfg(feature = "dns")]
use futures::{future::select_ok, FutureExt};
#[cfg(feature = "dns")]
use hickory_resolver::{
    config::LookupIpStrategy, name_server::TokioConnectionProvider, IntoName, TokioAsyncResolver,
};
#[cfg(feature = "dns")]
use log::debug;
use sasl::common::ChannelBinding;
use std::net::{IpAddr, SocketAddr};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use xmpp_parsers::jid::Jid;

use crate::xmpp_stream::XMPPStream;
use crate::Error;

#[cfg(feature = "starttls")]
pub mod starttls;
#[cfg(feature = "insecure-tcp")]
pub mod tcp;

/// trait returned wrapped in XMPPStream by ServerConnector
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
    ) -> impl std::future::Future<Output = Result<XMPPStream<Self::Stream>, Error>> + Send;

    /// Return channel binding data if available
    /// do not fail if channel binding is simply unavailable, just return Ok(None)
    /// this should only be called after the TLS handshake is finished
    fn channel_binding(_stream: &Self::Stream) -> Result<ChannelBinding, Error> {
        Ok(ChannelBinding::None)
    }
}

/// A simple wrapper to build [`TcpStream`]
pub struct Tcp;

impl Tcp {
    /// Connect directly to an IP/Port combo
    pub async fn connect(ip: IpAddr, port: u16) -> Result<TcpStream, Error> {
        Ok(TcpStream::connect(&SocketAddr::new(ip, port)).await?)
    }

    /// Connect over TCP, resolving A/AAAA records (happy eyeballs)
    #[cfg(feature = "dns")]
    pub async fn resolve(domain: &str, port: u16) -> Result<TcpStream, Error> {
        let ascii_domain = idna::domain_to_ascii(&domain)?;

        if let Ok(ip) = ascii_domain.parse() {
            return Ok(TcpStream::connect(&SocketAddr::new(ip, port)).await?);
        }

        let (config, mut options) = hickory_resolver::system_conf::read_system_conf()?;
        options.ip_strategy = LookupIpStrategy::Ipv4AndIpv6;
        let resolver = TokioAsyncResolver::new(config, options, TokioConnectionProvider::default());

        let ips = resolver.lookup_ip(ascii_domain).await?;

        // Happy Eyeballs: connect to all records in parallel, return the
        // first to succeed
        select_ok(
            ips.into_iter()
                .map(|ip| TcpStream::connect(SocketAddr::new(ip, port)).boxed()),
        )
        .await
        .map(|(result, _)| result)
        .map_err(|_| Error::Disconnected)
    }

    /// Connect over TCP, resolving SRV records
    #[cfg(feature = "dns")]
    pub async fn resolve_with_srv(
        domain: &str,
        srv: &str,
        fallback_port: u16,
    ) -> Result<TcpStream, Error> {
        let ascii_domain = idna::domain_to_ascii(&domain)?;

        if let Ok(ip) = ascii_domain.parse() {
            debug!("Attempting connection to {ip}:{fallback_port}");
            return Ok(TcpStream::connect(&SocketAddr::new(ip, fallback_port)).await?);
        }

        let resolver = TokioAsyncResolver::tokio_from_system_conf()?;

        let srv_domain = format!("{}.{}.", srv, ascii_domain).into_name()?;
        let srv_records = resolver.srv_lookup(srv_domain.clone()).await.ok();

        match srv_records {
            Some(lookup) => {
                // TODO: sort lookup records by priority/weight
                for srv in lookup.iter() {
                    debug!("Attempting connection to {srv_domain} {srv}");
                    match Self::resolve(&srv.target().to_ascii(), srv.port()).await {
                        Ok(stream) => return Ok(stream),
                        Err(_) => {}
                    }
                }
                Err(Error::Disconnected)
            }
            None => {
                // SRV lookup error, retry with hostname
                debug!("Attempting connection to {domain}:{fallback_port}");
                Self::resolve(domain, fallback_port).await
            }
        }
    }
}
