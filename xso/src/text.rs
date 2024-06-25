// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Module containing implementations for conversions to/from XML text.

use crate::{error::Error, FromXmlText, IntoXmlText};

#[cfg(feature = "jid")]
use jid;
#[cfg(feature = "uuid")]
use uuid;

macro_rules! convert_via_fromstr_and_display {
    ($($(#[cfg(feature = $feature:literal)])?$t:ty,)+) => {
        $(
            $(
                #[cfg(feature = $feature)]
                #[cfg_attr(docsrs, doc(cfg(feature = $feature)))]
            )?
            impl FromXmlText for $t {
                fn from_xml_text(s: String) -> Result<Self, Error> {
                    s.parse().map_err(Error::text_parse_error)
                }
            }

            $(
                #[cfg(feature = $feature)]
                #[cfg_attr(docsrs, doc(cfg(feature = $feature)))]
            )?
            impl IntoXmlText for $t {
                fn into_xml_text(self) -> Result<String, Error> {
                    Ok(self.to_string())
                }
            }
        )+
    }
}

/// This provides an implementation compliant with xsd::bool.
impl FromXmlText for bool {
    fn from_xml_text(s: String) -> Result<Self, Error> {
        match s.as_str() {
            "1" => "true",
            "0" => "false",
            other => other,
        }
        .parse()
        .map_err(Error::text_parse_error)
    }
}

/// This provides an implementation compliant with xsd::bool.
impl IntoXmlText for bool {
    fn into_xml_text(self) -> Result<String, Error> {
        Ok(self.to_string())
    }
}

convert_via_fromstr_and_display! {
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    f32,
    f64,
    std::net::IpAddr,
    std::net::Ipv4Addr,
    std::net::Ipv6Addr,
    std::net::SocketAddr,
    std::net::SocketAddrV4,
    std::net::SocketAddrV6,
    std::num::NonZeroU8,
    std::num::NonZeroU16,
    std::num::NonZeroU32,
    std::num::NonZeroU64,
    std::num::NonZeroU128,
    std::num::NonZeroUsize,
    std::num::NonZeroI8,
    std::num::NonZeroI16,
    std::num::NonZeroI32,
    std::num::NonZeroI64,
    std::num::NonZeroI128,
    std::num::NonZeroIsize,

    #[cfg(feature = "uuid")]
    uuid::Uuid,

    #[cfg(feature = "jid")]
    jid::Jid,
    #[cfg(feature = "jid")]
    jid::FullJid,
    #[cfg(feature = "jid")]
    jid::BareJid,
}
