// Copyright (c) 2017-2018 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::error::Error as StdError;
use std::fmt;

/// Contains one of the potential errors triggered while parsing an
/// [Element](../struct.Element.html) into a specialised struct.
#[derive(Debug)]
pub enum Error {
    /// The usual error when parsing something.
    ///
    /// TODO: use a structured error so the user can report it better, instead
    /// of a freeform string.
    ParseError(&'static str),

    /// Element local-name/namespace mismatch
    ///
    /// Returns the original element unaltered, as well as the expected ns and
    /// local-name.
    TypeMismatch(&'static str, &'static str, crate::Element),

    /// Generated when some base64 content fails to decode, usually due to
    /// extra characters.
    Base64Error(base64::DecodeError),

    /// Generated when text which should be an integer fails to parse.
    ParseIntError(std::num::ParseIntError),

    /// Generated when text which should be a string fails to parse.
    ParseStringError(std::string::ParseError),

    /// Generated when text which should be an IP address (IPv4 or IPv6) fails
    /// to parse.
    ParseAddrError(std::net::AddrParseError),

    /// Generated when text which should be a [JID](../../jid/struct.Jid.html)
    /// fails to parse.
    JidParseError(jid::Error),

    /// Generated when text which should be a
    /// [DateTime](../date/struct.DateTime.html) fails to parse.
    ChronoParseError(chrono::ParseError),
}

impl Error {
    /// Converts the TypeMismatch error to a generic ParseError
    ///
    /// This must be used when TryFrom is called on children to avoid confusing
    /// user code which assumes that TypeMismatch refers to the top level
    /// element only.
    pub(crate) fn hide_type_mismatch(self) -> Self {
        match self {
            Error::TypeMismatch(..) => Error::ParseError("Unexpected child element"),
            other => other,
        }
    }
}

impl StdError for Error {
    fn cause(&self) -> Option<&dyn StdError> {
        match self {
            Error::ParseError(_) | Error::TypeMismatch(..) => None,
            Error::Base64Error(e) => Some(e),
            Error::ParseIntError(e) => Some(e),
            Error::ParseStringError(e) => Some(e),
            Error::ParseAddrError(e) => Some(e),
            Error::JidParseError(e) => Some(e),
            Error::ChronoParseError(e) => Some(e),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ParseError(s) => write!(fmt, "parse error: {}", s),
            Error::TypeMismatch(ns, localname, element) => write!(
                fmt,
                "element type mismatch: expected {{{}}}{}, got {{{}}}{}",
                ns,
                localname,
                element.ns(),
                element.name()
            ),
            Error::Base64Error(e) => write!(fmt, "base64 error: {}", e),
            Error::ParseIntError(e) => write!(fmt, "integer parsing error: {}", e),
            Error::ParseStringError(e) => write!(fmt, "string parsing error: {}", e),
            Error::ParseAddrError(e) => write!(fmt, "IP address parsing error: {}", e),
            Error::JidParseError(e) => write!(fmt, "JID parsing error: {}", e),
            Error::ChronoParseError(e) => write!(fmt, "time parsing error: {}", e),
        }
    }
}

impl From<base64::DecodeError> for Error {
    fn from(err: base64::DecodeError) -> Error {
        Error::Base64Error(err)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Error {
        Error::ParseIntError(err)
    }
}

impl From<std::string::ParseError> for Error {
    fn from(err: std::string::ParseError) -> Error {
        Error::ParseStringError(err)
    }
}

impl From<std::net::AddrParseError> for Error {
    fn from(err: std::net::AddrParseError) -> Error {
        Error::ParseAddrError(err)
    }
}

impl From<jid::Error> for Error {
    fn from(err: jid::Error) -> Error {
        Error::JidParseError(err)
    }
}

impl From<chrono::ParseError> for Error {
    fn from(err: chrono::ParseError) -> Error {
        Error::ChronoParseError(err)
    }
}
