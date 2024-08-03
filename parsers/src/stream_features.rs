// Copyright (c) 2024 xmpp-rs contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use xso::{AsXml, FromXml};

use crate::ns;

/// Wraps `<stream:features/>`, usually the very first nonza of a
/// XMPP stream. Indicates which features are supported.
#[derive(FromXml, AsXml, PartialEq, Debug, Default, Clone)]
#[xml(namespace = ns::STREAM, name = "features")]
pub struct StreamFeatures {
    /// StartTLS is supported, and may be mandatory.
    #[xml(child(default))]
    pub starttls: Option<StartTls>,

    /// Bind is supported.
    #[xml(child(default))]
    pub bind: Option<Bind>,

    /// List of supported SASL mechanisms
    #[xml(child(default))]
    pub sasl_mechanisms: SaslMechanisms,
}

/// StartTLS is supported, and may be mandatory.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::TLS, name = "starttls")]
pub struct StartTls {
    /// Marker for mandatory StartTLS.
    #[xml(child(default))]
    pub required: Option<RequiredStartTls>,
}

/// Marker for mandatory StartTLS.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::TLS, name = "required")]
pub struct RequiredStartTls;

/// Bind is supported.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::BIND, name = "bind")]
pub struct Bind;

/// List of supported SASL mechanisms
#[derive(FromXml, AsXml, PartialEq, Debug, Clone, Default)]
#[xml(namespace = ns::SASL, name = "mechanisms")]
pub struct SaslMechanisms {
    /// List of information elements describing this avatar.
    #[xml(child(n = ..))]
    pub mechanisms: Vec<SaslMechanism>,
}

/// The name of a SASL mechanism.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::SASL, name = "mechanism")]
pub struct SaslMechanism {
    /// The stringy name of the mechanism.
    #[xml(text)]
    pub mechanism: String,
}

impl StreamFeatures {
    /// Can initiate TLS session with this server?
    pub fn can_starttls(&self) -> bool {
        self.starttls.is_some()
    }

    /// Does server support user resource binding?
    pub fn can_bind(&self) -> bool {
        self.bind.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minidom::Element;

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn test_size() {
        assert_size!(SaslMechanism, 12);
        assert_size!(SaslMechanisms, 12);
        assert_size!(Bind, 0);
        assert_size!(RequiredStartTls, 0);
        assert_size!(StartTls, 1);
        assert_size!(StreamFeatures, 16);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_size() {
        assert_size!(SaslMechanism, 24);
        assert_size!(SaslMechanisms, 24);
        assert_size!(Bind, 0);
        assert_size!(RequiredStartTls, 0);
        assert_size!(StartTls, 1);
        assert_size!(StreamFeatures, 32);
    }

    #[test]
    fn test_required_starttls() {
        let elem: Element = "<stream:features xmlns:stream='http://etherx.jabber.org/streams'>
                                 <starttls xmlns='urn:ietf:params:xml:ns:xmpp-tls'>
                                     <required/>
                                 </starttls>
                             </stream:features>"
            .parse()
            .unwrap();

        let features = StreamFeatures::try_from(elem).unwrap();

        assert_eq!(features.can_bind(), false);
        assert_eq!(features.sasl_mechanisms.mechanisms.len(), 0);
        assert_eq!(features.can_starttls(), true);
        assert_eq!(features.starttls.unwrap().required.is_some(), true);
    }

    #[test]
    // TODO: Unignore me when xso supports collections of unknown children, see
    // https://gitlab.com/xmpp-rs/xmpp-rs/-/issues/136
    #[ignore]
    fn test_deprecated_compression() {
        let elem: Element = "<stream:features xmlns:stream='http://etherx.jabber.org/streams'>
                                 <bind xmlns='urn:ietf:params:xml:ns:xmpp-bind'/>
                                 <compression xmlns='http://jabber.org/features/compress'>
                                     <method>zlib</method>
                                     <method>lzw</method>
                                 </compression>
                             </stream:features>"
            .parse()
            .unwrap();

        let features = StreamFeatures::try_from(elem).unwrap();

        assert_eq!(features.can_bind(), true);
        assert_eq!(features.sasl_mechanisms.mechanisms.len(), 0);
        assert_eq!(features.can_starttls(), false);
    }

    #[test]
    fn test_empty_features() {
        let elem: Element = "<stream:features xmlns:stream='http://etherx.jabber.org/streams'/>"
            .parse()
            .unwrap();

        let features = StreamFeatures::try_from(elem).unwrap();

        assert_eq!(features.can_bind(), false);
        assert_eq!(features.sasl_mechanisms.mechanisms.len(), 0);
        assert_eq!(features.can_starttls(), false);
    }
}
