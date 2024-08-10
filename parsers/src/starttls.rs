// Copyright (c) 2018 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use xso::{AsXml, FromXml};

use crate::ns;

/// Request to start TLS.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::TLS, name = "starttls")]
pub struct Request;

/// Information that TLS may now commence.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::TLS, name = "proceed")]
pub struct Proceed;

/// Stream feature for StartTLS
///
/// Used in [`crate::stream_features::StreamFeatures`].
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::TLS, name = "starttls")]
pub struct StartTls {
    /// Marker for mandatory StartTLS.
    // TODO: replace with `#[xml(flag)]` once we have it
    #[xml(child(default))]
    pub required: Option<RequiredStartTls>,
}

/// Marker for mandatory StartTLS.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::TLS, name = "required")]
pub struct RequiredStartTls;

/// Enum which allows parsing/serialising any STARTTLS element.
#[derive(FromXml, AsXml, Debug, Clone)]
#[xml()]
pub enum Nonza {
    /// Request to start TLS
    #[xml(transparent)]
    Request(Request),

    /// Information that TLS may now commence
    #[xml(transparent)]
    Proceed(Proceed),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn test_size() {
        assert_size!(RequiredStartTls, 0);
        assert_size!(StartTls, 1);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_size() {
        assert_size!(RequiredStartTls, 0);
        assert_size!(StartTls, 1);
    }
}
