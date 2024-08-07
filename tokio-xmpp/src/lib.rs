//! Low-level [XMPP](https://xmpp.org/) decentralized instant messaging & social networking implementation with asynchronous I/O using [tokio](https://tokio.rs/).
//!
//! For an easier, batteries-included experience, try the [xmpp crate](https://docs.rs/xmpp).
//!
//! # Getting started
//!
//! In most cases, you want to start with a [`Client`], that will connect to a server over TCP/IP with StartTLS encryption. Then, you can build an event loop by calling the client's `next` method repeatedly. You can find a more complete example in the [examples/echo_bot.rs](https://gitlab.com/xmpp-rs/xmpp-rs/-/blob/main/tokio-xmpp/examples/echo_bot.rs) file in the repository.
//!
//! # Features
//!
//! This library is not feature-complete yet. Here's a quick overview of the feature set.
//!
//! Supported implementations:
//! - [x] Clients
//! - [x] Components
//! - [ ] Servers
//!
//! Supported transports:
//! - [x] Plaintext TCP (IPv4/IPv6)
//! - [x] StartTLS TCP (IPv4/IPv6 with [happy eyeballs](https://en.wikipedia.org/wiki/Happy_Eyeballs) support)
//! - [x] Custom connectors via the [`connect::ServerConnector`] trait
//! - [ ] Websockets
//! - [ ] BOSH
//!
//! # More information
//!
//! You can find more information on our website [xmpp.rs](https://xmpp.rs/) or by joining our chatroom [chat@xmpp.rs](xmpp:chat@xmpp.rs?join).

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

mod event;
pub use event::Event;
pub mod connect;
pub mod proto;
pub mod xmlstream;

mod client;
pub use client::Client;

#[cfg(feature = "insecure-tcp")]
mod component;
#[cfg(feature = "insecure-tcp")]
pub use crate::component::Component;

/// Detailed error types
pub mod error;

#[doc(inline)]
/// Generic tokio_xmpp Error
pub use crate::error::Error;

// Re-exports
pub use minidom;
pub use xmpp_parsers as parsers;
pub use xmpp_parsers::jid;
