//! `starttls::ServerConfig` provides a `ServerConnector` for starttls connections

#[cfg(feature = "tls-native")]
use native_tls::Error as TlsError;
use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt;
use std::io;
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

use sasl::common::ChannelBinding;
use tokio::{
    io::{AsyncRead, AsyncWrite, BufStream},
    net::TcpStream,
};
use xmpp_parsers::{
    jid::Jid,
    starttls::{self, Request},
};

use crate::{
    connect::{DnsConfig, ServerConnector, ServerConnectorError},
    error::{Error, ProtocolError},
    xmlstream::{
        initiate_stream, PendingFeaturesRecv, ReadError, StreamHeader, XmppStream,
        XmppStreamElement,
    },
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
    type Stream = BufStream<TlsStream<TcpStream>>;

    async fn connect(
        &self,
        jid: &Jid,
        ns: &'static str,
    ) -> Result<PendingFeaturesRecv<Self::Stream>, Error> {
        let tcp_stream = tokio::io::BufStream::new(self.0.resolve().await?);

        // Unencryped XmppStream
        let xmpp_stream = initiate_stream(
            tcp_stream,
            ns,
            StreamHeader {
                to: Some(Cow::Borrowed(jid.domain().as_str())),
                from: None,
                id: None,
            },
        )
        .await?;
        let (features, xmpp_stream) = xmpp_stream.recv_features().await?;

        if features.can_starttls() {
            // TlsStream
            let tls_stream = starttls(xmpp_stream, jid.domain().as_str()).await?;
            // Encrypted XmppStream
            Ok(initiate_stream(
                tokio::io::BufStream::new(tls_stream),
                ns,
                StreamHeader {
                    to: Some(Cow::Borrowed(jid.domain().as_str())),
                    from: None,
                    id: None,
                },
            )
            .await?)
        } else {
            Err(crate::Error::Protocol(ProtocolError::NoTls).into())
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
            let (_, connection) = stream.get_ref().get_ref();
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
    xmpp_stream: XmppStream<BufStream<S>>,
    domain: &str,
) -> Result<TlsStream<S>, Error> {
    let domain = domain.to_owned();
    let stream = xmpp_stream.into_inner().into_inner();
    let tls_stream = TlsConnector::from(NativeTlsConnector::builder().build().unwrap())
        .connect(&domain, stream)
        .await
        .map_err(|e| StartTlsError::Tls(e))?;
    Ok(tls_stream)
}

#[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
async fn get_tls_stream<S: AsyncRead + AsyncWrite + Unpin>(
    xmpp_stream: XmppStream<BufStream<S>>,
    domain: &str,
) -> Result<TlsStream<S>, Error> {
    let domain = ServerName::try_from(domain.to_owned()).map_err(StartTlsError::DnsNameError)?;
    let stream = xmpp_stream.into_inner().into_inner();
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
    mut stream: XmppStream<BufStream<S>>,
    domain: &str,
) -> Result<TlsStream<S>, Error> {
    stream
        .send(&XmppStreamElement::Starttls(starttls::Nonza::Request(
            Request,
        )))
        .await?;

    loop {
        match stream.next().await {
            Some(Ok(XmppStreamElement::Starttls(starttls::Nonza::Proceed(_)))) => {
                break;
            }
            Some(Ok(_)) => (),
            Some(Err(ReadError::SoftTimeout)) => (),
            Some(Err(ReadError::HardError(e))) => return Err(e.into()),
            Some(Err(ReadError::ParseError(e))) => {
                return Err(io::Error::new(io::ErrorKind::InvalidData, e).into())
            }
            None | Some(Err(ReadError::StreamFooterReceived)) => {
                return Err(crate::Error::Disconnected)
            }
        }
    }

    get_tls_stream(stream, domain).await
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
