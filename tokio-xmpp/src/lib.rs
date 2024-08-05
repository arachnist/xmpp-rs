//! XMPP implementation with asynchronous I/O using Tokio.

#![deny(unsafe_code, missing_docs, bare_trait_objects)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

#[cfg(all(
    not(xmpprs_doc_build),
    not(doc),
    feature = "tls-native",
    feature = "tls-rust"
))]
compile_error!("Both tls-native and tls-rust features can't be enabled at the same time.");

#[cfg(all(
    feature = "starttls",
    not(feature = "tls-native"),
    not(feature = "tls-rust")
))]
compile_error!(
    "when starttls feature enabled one of tls-native and tls-rust features must be enabled."
);

#[cfg(feature = "starttls")]
pub mod starttls;
mod stream_start;
mod xmpp_codec;
pub use crate::xmpp_codec::{Packet, XmppCodec};
mod event;
pub use event::Event;
mod client;
pub mod connect;
pub mod xmpp_stream;

pub use client::{
    async_client::{Client as AsyncClient, Config as AsyncConfig},
};
mod component;
pub use crate::component::Component;
/// Detailed error types
pub mod error;
/// Generic tokio_xmpp Error
pub use crate::error::Error;

// Re-exports
pub use minidom;
pub use xmpp_parsers as parsers;
pub use xmpp_parsers::jid;
