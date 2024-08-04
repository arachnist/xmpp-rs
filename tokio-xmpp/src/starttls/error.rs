//! StartTLS ServerConnector Error

use hickory_resolver::{error::ResolveError, proto::error::ProtoError};
#[cfg(feature = "tls-native")]
use native_tls::Error as TlsError;
use std::error::Error as StdError;
use std::fmt;
#[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
use tokio_rustls::rustls::pki_types::InvalidDnsNameError;
#[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
use tokio_rustls::rustls::Error as TlsError;

/// StartTLS ServerConnector Error
#[derive(Debug)]
pub enum Error {
    /// DNS protocol error
    Dns(ProtoError),
    /// DNS resolution error
    Resolve(ResolveError),
    /// DNS label conversion error, no details available from module
    /// `idna`
    Idna,
    /// TLS error
    Tls(TlsError),
    #[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
    /// DNS name parsing error
    DnsNameError(InvalidDnsNameError),
    /// tokio-xmpp error
    TokioXMPP(crate::error::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Dns(e) => write!(fmt, "{:?}", e),
            Error::Resolve(e) => write!(fmt, "{:?}", e),
            Error::Idna => write!(fmt, "IDNA error"),
            Error::Tls(e) => write!(fmt, "TLS error: {}", e),
            #[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
            Error::DnsNameError(e) => write!(fmt, "DNS name error: {}", e),
            Error::TokioXMPP(e) => write!(fmt, "TokioXMPP error: {}", e),
        }
    }
}

impl StdError for Error {}

impl From<crate::error::Error> for Error {
    fn from(e: crate::error::Error) -> Self {
        Error::TokioXMPP(e)
    }
}

impl From<TlsError> for Error {
    fn from(e: TlsError) -> Self {
        Error::Tls(e)
    }
}

#[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
impl From<InvalidDnsNameError> for Error {
    fn from(e: InvalidDnsNameError) -> Self {
        Error::DnsNameError(e)
    }
}
