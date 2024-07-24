// Copyright (c) 2024 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::mam;
use crate::ns;
use minidom::Element;
use xso::error::{Error, FromElementError};

/// Represents the `<bind/>` element, as sent by the server in SASL 2 to advertise which features
/// can be enabled during the binding step.
#[derive(Debug, Clone, PartialEq)]
pub struct BindFeature {
    /// The features that can be enabled by the client.
    pub inline_features: Vec<String>,
}

impl TryFrom<Element> for BindFeature {
    type Error = FromElementError;

    fn try_from(root: Element) -> Result<BindFeature, Self::Error> {
        check_self!(root, "bind", BIND2);
        check_no_attributes!(root, "bind");

        let mut inline = None;
        for child in root.children() {
            if child.is("inline", ns::BIND2) {
                if inline.is_some() {
                    return Err(
                        Error::Other("Bind must not have more than one inline element.").into(),
                    );
                }
                check_no_attributes!(child, "inline");
                inline = Some(child);
            } else {
                return Err(Error::Other("Unknown element in Bind.").into());
            }
        }

        let mut inline_features = Vec::new();
        if let Some(inline) = inline {
            for child in inline.children() {
                if child.is("feature", ns::BIND2) {
                    check_no_children!(child, "feature");
                    check_no_unknown_attributes!(child, "feature", ["var"]);
                    let var = get_attr!(child, "var", Required);
                    inline_features.push(var);
                } else {
                    return Err(Error::Other("Unknown element in Inline.").into());
                }
            }
        }

        Ok(BindFeature { inline_features })
    }
}

impl From<BindFeature> for Element {
    fn from(bind: BindFeature) -> Element {
        Element::builder("bind", ns::BIND2)
            .append_all(if bind.inline_features.is_empty() {
                None
            } else {
                Some(
                    Element::builder("inline", ns::BIND2).append_all(
                        bind.inline_features
                            .into_iter()
                            .map(|var| Element::builder("feature", ns::BIND2).attr("var", var)),
                    ),
                )
            })
            .build()
    }
}

/// Represents a `<bind/>` element, as sent by the client inline in the `<authenticate/>` SASL 2
/// element, to perform the binding at the same time as the authentication.
#[derive(Debug, Clone, PartialEq)]
pub struct BindQuery {
    /// Short text string that typically identifies the software the user is using, mostly useful
    /// for diagnostic purposes for users, operators and developers.  This tag may be visible to
    /// other entities on the XMPP network.
    pub tag: Option<String>,

    /// Features that the client requests to be automatically enabled for its new session.
    pub payloads: Vec<Element>,
}

impl TryFrom<Element> for BindQuery {
    type Error = FromElementError;

    fn try_from(root: Element) -> Result<BindQuery, Self::Error> {
        check_self!(root, "bind", BIND2);
        check_no_attributes!(root, "bind");

        let mut tag = None;
        let mut payloads = Vec::new();
        for child in root.children() {
            if child.is("tag", ns::BIND2) {
                if tag.is_some() {
                    return Err(
                        Error::Other("Bind must not have more than one tag element.").into(),
                    );
                }
                check_no_attributes!(child, "tag");
                check_no_children!(child, "tag");
                tag = Some(child.text());
            } else {
                payloads.push(child.clone());
            }
        }

        Ok(BindQuery { tag, payloads })
    }
}

impl From<BindQuery> for Element {
    fn from(bind: BindQuery) -> Element {
        Element::builder("bind", ns::BIND2)
            .append_all(
                bind.tag
                    .map(|tag| Element::builder("tag", ns::BIND2).append(tag)),
            )
            .append_all(bind.payloads)
            .build()
    }
}

/// Represents a `<bound/>` element, which tells the client its resource is bound, alongside other
/// requests.
#[derive(Debug, Clone, PartialEq)]
pub struct Bound {
    /// Indicates which messages got missed by this particular device, start is the oldest message
    /// and end is the newest, before this connection.
    pub mam_metadata: Option<mam::MetadataResponse>,

    /// Additional payloads which happened during the binding process.
    pub payloads: Vec<Element>,
}

impl TryFrom<Element> for Bound {
    type Error = FromElementError;

    fn try_from(root: Element) -> Result<Bound, Self::Error> {
        check_self!(root, "bound", BIND2);
        check_no_attributes!(root, "bound");

        let mut mam_metadata = None;
        let mut payloads = Vec::new();
        for child in root.children() {
            if child.is("metadata", ns::MAM) {
                if mam_metadata.is_some() {
                    return Err(
                        Error::Other("Bind must not have more than one metadata element.").into(),
                    );
                }
                mam_metadata = Some(mam::MetadataResponse::try_from(child.clone())?);
            } else {
                payloads.push(child.clone());
            }
        }

        Ok(Bound {
            mam_metadata,
            payloads,
        })
    }
}

impl From<Bound> for Element {
    fn from(bound: Bound) -> Element {
        Element::builder("bound", ns::BIND2)
            .append_all(bound.mam_metadata)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn test_size() {
        assert_size!(BindFeature, 12);
        assert_size!(BindQuery, 24);
        assert_size!(Bound, 68);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_size() {
        assert_size!(BindFeature, 24);
        assert_size!(BindQuery, 48);
        assert_size!(Bound, 104);
    }

    #[test]
    fn test_simple() {
        // Example 1
        let elem: Element =
            "<bind xmlns='urn:xmpp:bind:0'><inline><feature var='urn:xmpp:carbons:2'/><feature var='urn:xmpp:csi:0'/><feature var='urn:xmpp:sm:3'/></inline></bind>"
                .parse()
                .unwrap();
        let bind = BindFeature::try_from(elem.clone()).unwrap();
        assert_eq!(bind.inline_features.len(), 3);
        assert_eq!(bind.inline_features[0], "urn:xmpp:carbons:2");
        assert_eq!(bind.inline_features[1], "urn:xmpp:csi:0");
        assert_eq!(bind.inline_features[2], "urn:xmpp:sm:3");
        let elem2 = bind.into();
        assert_eq!(elem, elem2);

        // Example 2
        let elem: Element = "<bind xmlns='urn:xmpp:bind:0'><tag>AwesomeXMPP</tag></bind>"
            .parse()
            .unwrap();
        let bind = BindQuery::try_from(elem).unwrap();
        assert_eq!(bind.tag.unwrap(), "AwesomeXMPP");
        assert_eq!(bind.payloads.len(), 0);

        // Example 3
        let elem: Element = "<bind xmlns='urn:xmpp:bind:0'><tag>AwesomeXMPP</tag><enable xmlns='urn:xmpp:carbons:2'/><enable xmlns='urn:xmpp:sm:3'/><inactive xmlns='urn:xmpp:csi:0'/></bind>".parse().unwrap();
        let bind = BindQuery::try_from(elem).unwrap();
        assert_eq!(bind.tag.unwrap(), "AwesomeXMPP");
        assert_eq!(bind.payloads.len(), 3);

        // Example 4
        let elem: Element = "<bound xmlns='urn:xmpp:bind:0'><metadata xmlns='urn:xmpp:mam:2'><start id='YWxwaGEg' timestamp='2008-08-22T21:09:04Z'/><end id='b21lZ2Eg' timestamp='2020-04-20T14:34:21Z'/></metadata></bound>".parse().unwrap();
        let bound = Bound::try_from(elem).unwrap();
        assert!(bound.mam_metadata.is_some());
        assert_eq!(bound.payloads.len(), 0);
    }
}
