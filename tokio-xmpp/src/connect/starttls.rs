//! `starttls::ServerConfig` provides a `ServerConnector` for starttls connections

#[cfg(feature = "tls-native")]
use native_tls::Error as TlsError;
use std::error::Error as StdError;
use std::fmt;
#[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
use tokio_rustls::rustls::pki_types::InvalidDnsNameError;
#[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
use tokio_rustls::rustls::Error as TlsError;

use futures::{sink::SinkExt, stream::StreamExt};

#[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
use {
    std::sync::Arc,
    tokio_rustls::{
        client::TlsStream,
        rustls::pki_types::ServerName,
        rustls::{ClientConfig, RootCertStore},
        TlsConnector,
    },
};

#[cfg(feature = "tls-native")]
use {
    native_tls::TlsConnector as NativeTlsConnector,
    tokio_native_tls::{TlsConnector, TlsStream},
};

use minidom::Element;
use sasl::common::ChannelBinding;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use xmpp_parsers::{jid::Jid, ns};

use crate::{
    connect::{DnsConfig, ServerConnector, ServerConnectorError},
    error::{Error, ProtocolError},
    proto::{Packet, XmppStream},
    Client,
};

/// Client that connects over StartTls
pub type StartTlsClient = Client<StartTlsServerConnector>;

/// Connect via TCP+StartTLS to an XMPP server
#[derive(Debug, Clone)]
pub struct StartTlsServerConnector(pub DnsConfig);

impl From<DnsConfig> for StartTlsServerConnector {
    fn from(dns_config: DnsConfig) -> StartTlsServerConnector {
        Self(dns_config)
    }
}

impl ServerConnector for StartTlsServerConnector {
    type Stream = TlsStream<TcpStream>;
    async fn connect(&self, jid: &Jid, ns: &str) -> Result<XmppStream<Self::Stream>, Error> {
        let tcp_stream = self.0.resolve().await?;

        // Unencryped XmppStream
        let xmpp_stream = XmppStream::start(tcp_stream, jid.clone(), ns.to_owned()).await?;

        if xmpp_stream.stream_features.can_starttls() {
            // TlsStream
            let tls_stream = starttls(xmpp_stream).await?;
            // Encrypted XmppStream
            Ok(XmppStream::start(tls_stream, jid.clone(), ns.to_owned()).await?)
        } else {
            return Err(crate::Error::Protocol(ProtocolError::NoTls).into());
        }
    }

    fn channel_binding(
        #[allow(unused_variables)] stream: &Self::Stream,
    ) -> Result<sasl::common::ChannelBinding, Error> {
        #[cfg(feature = "tls-native")]
        {
            log::warn!("tls-native doesn’t support channel binding, please use tls-rust if you want this feature!");
            Ok(ChannelBinding::None)
        }
        #[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
        {
            let (_, connection) = stream.get_ref();
            Ok(match connection.protocol_version() {
                // TODO: Add support for TLS 1.2 and earlier.
                Some(tokio_rustls::rustls::ProtocolVersion::TLSv1_3) => {
                    let data = vec![0u8; 32];
                    let data = connection
                        .export_keying_material(data, b"EXPORTER-Channel-Binding", None)
                        .map_err(|e| StartTlsError::Tls(e))?;
                    ChannelBinding::TlsExporter(data)
                }
                _ => ChannelBinding::None,
            })
        }
    }
}

#[cfg(feature = "tls-native")]
async fn get_tls_stream<S: AsyncRead + AsyncWrite + Unpin>(
    xmpp_stream: XmppStream<S>,
) -> Result<TlsStream<S>, Error> {
    let domain = xmpp_stream.jid.domain().to_owned();
    let stream = xmpp_stream.into_inner();
    let tls_stream = TlsConnector::from(NativeTlsConnector::builder().build().unwrap())
        .connect(&domain, stream)
        .await
        .map_err(|e| StartTlsError::Tls(e))?;
    Ok(tls_stream)
}

#[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
async fn get_tls_stream<S: AsyncRead + AsyncWrite + Unpin>(
    xmpp_stream: XmppStream<S>,
) -> Result<TlsStream<S>, Error> {
    let domain = xmpp_stream.jid.domain().to_string();
    let domain = ServerName::try_from(domain).map_err(|e| StartTlsError::DnsNameError(e))?;
    let stream = xmpp_stream.into_inner();
    let mut root_store = RootCertStore::empty();
    #[cfg(feature = "webpki-roots")]
    {
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    }
    #[cfg(feature = "rustls-native-certs")]
    {
        root_store.add_parsable_certificates(rustls_native_certs::load_native_certs()?);
    }
    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let tls_stream = TlsConnector::from(Arc::new(config))
        .connect(domain, stream)
        .await
        .map_err(|e| Error::from(crate::Error::Io(e)))?;
    Ok(tls_stream)
}

/// Performs `<starttls/>` on an XmppStream and returns a binary
/// TlsStream.
pub async fn starttls<S: AsyncRead + AsyncWrite + Unpin>(
    mut xmpp_stream: XmppStream<S>,
) -> Result<TlsStream<S>, Error> {
    let nonza = Element::builder("starttls", ns::TLS).build();
    let packet = Packet::Stanza(nonza);
    xmpp_stream.send(packet).await?;

    loop {
        match xmpp_stream.next().await {
            Some(Ok(Packet::Stanza(ref stanza))) if stanza.name() == "proceed" => break,
            Some(Ok(Packet::Text(_))) => {}
            Some(Err(e)) => return Err(e.into()),
            _ => {
                return Err(crate::Error::Protocol(ProtocolError::NoTls).into());
            }
        }
    }

    get_tls_stream(xmpp_stream).await
}

/// StartTLS ServerConnector Error
#[derive(Debug)]
pub enum StartTlsError {
    /// TLS error
    Tls(TlsError),
    #[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
    /// DNS name parsing error
    DnsNameError(InvalidDnsNameError),
}

impl ServerConnectorError for StartTlsError {}

impl fmt::Display for StartTlsError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Tls(e) => write!(fmt, "TLS error: {}", e),
            #[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
            Self::DnsNameError(e) => write!(fmt, "DNS name error: {}", e),
        }
    }
}

impl StdError for StartTlsError {}

impl From<TlsError> for StartTlsError {
    fn from(e: TlsError) -> Self {
        Self::Tls(e)
    }
}

#[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
impl From<InvalidDnsNameError> for StartTlsError {
    fn from(e: InvalidDnsNameError) -> Self {
        Self::DnsNameError(e)
    }
}
