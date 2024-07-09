// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Module containing implementations for conversions to/from XML text.

#[cfg(feature = "base64")]
use core::marker::PhantomData;

use std::borrow::Cow;

use crate::{error::Error, AsXmlText, FromXmlText, IntoXmlText};

#[cfg(feature = "base64")]
use base64::engine::{general_purpose::STANDARD as StandardBase64Engine, Engine as _};
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

            $(
                #[cfg(feature = $feature)]
                #[cfg_attr(docsrs, doc(cfg(feature = $feature)))]
            )?
            impl AsXmlText for $t {
                fn as_xml_text(&self) -> Result<Cow<'_, str>, Error> {
                    Ok(Cow::Owned(self.to_string()))
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

/// This provides an implementation compliant with xsd::bool.
impl AsXmlText for bool {
    fn as_xml_text(&self) -> Result<Cow<'_, str>, Error> {
        match self {
            true => Ok(Cow::Borrowed("true")),
            false => Ok(Cow::Borrowed("false")),
        }
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

/// Represent a way to encode/decode text data into a Rust type.
///
/// This trait can be used in scenarios where implementing [`FromXmlText`]
/// and/or [`IntoXmlText`] on a type is not feasible or sensible, such as the
/// following:
///
/// 1. The type originates in a foreign crate, preventing the implementation
///    of foreign traits.
///
/// 2. There is more than one way to convert a value to/from XML.
///
/// The codec to use for a text can be specified in the attributes understood
/// by `FromXml` and `IntoXml` derive macros. See the documentation of the
/// [`FromXml`][`macro@crate::FromXml`] derive macro for details.
pub trait TextCodec<T> {
    /// Decode a string value into the type.
    fn decode(s: String) -> Result<T, Error>;

    /// Encode the type as string value.
    ///
    /// If this returns `None`, the string value is not emitted at all.
    fn encode(value: T) -> Result<Option<String>, Error>;
}

/// Text codec which does no transform.
pub struct Plain;

impl TextCodec<String> for Plain {
    fn decode(s: String) -> Result<String, Error> {
        Ok(s)
    }

    fn encode(value: String) -> Result<Option<String>, Error> {
        Ok(Some(value))
    }
}

/// Text codec which returns None instead of the empty string.
pub struct EmptyAsNone;

impl TextCodec<Option<String>> for EmptyAsNone {
    fn decode(s: String) -> Result<Option<String>, Error> {
        if s.is_empty() {
            Ok(None)
        } else {
            Ok(Some(s))
        }
    }

    fn encode(value: Option<String>) -> Result<Option<String>, Error> {
        Ok(match value {
            Some(v) if !v.is_empty() => Some(v),
            Some(_) | None => None,
        })
    }
}

/// Trait for preprocessing text data from XML.
///
/// This may be used by codecs to allow to customize some of their behaviour.
pub trait TextFilter {
    /// Process the incoming string and return the result of the processing.
    fn preprocess(s: String) -> String;
}

/// Text preprocessor which returns the input unchanged.
pub struct NoFilter;

impl TextFilter for NoFilter {
    fn preprocess(s: String) -> String {
        s
    }
}

/// Text preprocessor to remove all whitespace.
pub struct StripWhitespace;

impl TextFilter for StripWhitespace {
    fn preprocess(s: String) -> String {
        let s: String = s
            .chars()
            .filter(|ch| *ch != ' ' && *ch != '\n' && *ch != '\t')
            .collect();
        s
    }
}

/// Text codec transforming text to binary using standard base64.
///
/// The `Filter` type argument can be used to employ additional preprocessing
/// of incoming text data. Most interestingly, passing [`StripWhitespace`]
/// will make the implementation ignore any whitespace within the text.
#[cfg(feature = "base64")]
#[cfg_attr(docsrs, doc(cfg(feature = "base64")))]
pub struct Base64<Filter: TextFilter = NoFilter>(PhantomData<Filter>);

#[cfg(feature = "base64")]
#[cfg_attr(docsrs, doc(cfg(feature = "base64")))]
impl<Filter: TextFilter> TextCodec<Vec<u8>> for Base64<Filter> {
    fn decode(s: String) -> Result<Vec<u8>, Error> {
        let value = Filter::preprocess(s);
        StandardBase64Engine
            .decode(value.as_bytes())
            .map_err(Error::text_parse_error)
    }

    fn encode(value: Vec<u8>) -> Result<Option<String>, Error> {
        Ok(Some(StandardBase64Engine.encode(&value)))
    }
}

#[cfg(feature = "base64")]
#[cfg_attr(docsrs, doc(cfg(feature = "base64")))]
impl<Filter: TextFilter> TextCodec<Option<Vec<u8>>> for Base64<Filter> {
    fn decode(s: String) -> Result<Option<Vec<u8>>, Error> {
        if s.is_empty() {
            return Ok(None);
        }
        Ok(Some(Self::decode(s)?))
    }

    fn encode(decoded: Option<Vec<u8>>) -> Result<Option<String>, Error> {
        decoded.map(Self::encode).transpose().map(Option::flatten)
    }
}
