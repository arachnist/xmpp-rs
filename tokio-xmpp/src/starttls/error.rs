//! StartTLS ServerConnector Error

#[cfg(feature = "tls-native")]
use native_tls::Error as TlsError;
use std::error::Error as StdError;
use std::fmt;
#[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
use tokio_rustls::rustls::pki_types::InvalidDnsNameError;
#[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
use tokio_rustls::rustls::Error as TlsError;

use super::ServerConnectorError;

/// StartTLS ServerConnector Error
#[derive(Debug)]
pub enum Error {
    /// TLS error
    Tls(TlsError),
    #[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
    /// DNS name parsing error
    DnsNameError(InvalidDnsNameError),
}

impl ServerConnectorError for Error {}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Tls(e) => write!(fmt, "TLS error: {}", e),
            #[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
            Self::DnsNameError(e) => write!(fmt, "DNS name error: {}", e),
        }
    }
}

impl StdError for Error {}

impl From<TlsError> for Error {
    fn from(e: TlsError) -> Self {
        Self::Tls(e)
    }
}

#[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
impl From<InvalidDnsNameError> for Error {
    fn from(e: InvalidDnsNameError) -> Self {
        Self::DnsNameError(e)
    }
}
