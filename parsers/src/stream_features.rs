// Copyright (c) 2024 xmpp-rs contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use minidom::Element;
use xso::{AsXml, FromXml};

use crate::bind::BindFeature;
use crate::ns;
use crate::sasl2::Authentication;
use crate::sasl_cb::SaslChannelBinding;
use crate::stream_limits::Limits;

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
    pub bind: Option<BindFeature>,

    /// List of supported SASL mechanisms
    #[xml(child(default))]
    pub sasl_mechanisms: SaslMechanisms,

    /// Limits advertised by the server.
    #[xml(child(default))]
    pub limits: Option<Limits>,

    /// Extensible SASL Profile, a newer authentication method than the one from the RFC.
    #[xml(child(default))]
    pub sasl2: Option<Authentication>,

    /// SASL Channel-Binding Type Capability.
    #[xml(child(default))]
    pub sasl_cb: Option<SaslChannelBinding>,

    /// Other stream features advertised
    ///
    /// If some features you use end up here, you may want to contribute
    /// a typed equivalent to the xmpp-parsers project!
    #[xml(element(n = ..))]
    pub others: Vec<Element>,
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

/// List of supported SASL mechanisms
#[derive(FromXml, AsXml, PartialEq, Debug, Clone, Default)]
#[xml(namespace = ns::SASL, name = "mechanisms")]
pub struct SaslMechanisms {
    /// List of information elements describing this SASL mechanism.
    #[xml(extract(n = .., name = "mechanism", fields(text(type_ = String))))]
    pub mechanisms: Vec<String>,
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
        assert_size!(SaslMechanisms, 12);
        assert_size!(RequiredStartTls, 0);
        assert_size!(StartTls, 1);
        assert_size!(StreamFeatures, 92);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_size() {
        assert_size!(SaslMechanisms, 24);
        assert_size!(RequiredStartTls, 0);
        assert_size!(StartTls, 1);
        assert_size!(StreamFeatures, 168);
    }

    #[test]
    fn test_sasl_mechanisms() {
        let elem: Element = "<stream:features xmlns:stream='http://etherx.jabber.org/streams'>
            <mechanisms xmlns='urn:ietf:params:xml:ns:xmpp-sasl'>
                <mechanism>PLAIN</mechanism>
                <mechanism>SCRAM-SHA-1</mechanism>
                <mechanism>SCRAM-SHA-1-PLUS</mechanism>
            </mechanisms>
        </stream:features>"
            .parse()
            .unwrap();

        let features = StreamFeatures::try_from(elem).unwrap();
        assert_eq!(
            features.sasl_mechanisms.mechanisms,
            ["PLAIN", "SCRAM-SHA-1", "SCRAM-SHA-1-PLUS"]
        );
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
        assert_eq!(features.others.len(), 1);

        let compression = &features.others[0];
        assert!(compression.is("compression", "http://jabber.org/features/compress"));
        let mut children = compression.children();

        let child = children.next().expect("zlib not found");
        assert_eq!(child.name(), "method");
        let mut texts = child.texts();
        assert_eq!(texts.next().unwrap(), "zlib");
        assert_eq!(texts.next(), None);

        let child = children.next().expect("lzw not found");
        assert_eq!(child.name(), "method");
        let mut texts = child.texts();
        assert_eq!(texts.next().unwrap(), "lzw");
        assert_eq!(texts.next(), None);
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
