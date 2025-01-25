// Copyright (c) 2020 Paul Fariello <paul@fariello.eu>
// Copyright (c) 2018 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use xso::{AsXml, FromXml};

use crate::data_forms::DataForm;
use crate::iq::{IqGetPayload, IqResultPayload, IqSetPayload};
use crate::ns;
use crate::pubsub::{AffiliationAttribute, NodeName, Subscription};
use jid::Jid;
use minidom::Element;
use xso::error::{Error, FromElementError};

/// A list of affiliations you have on a service, or on a node.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::PUBSUB_OWNER, name = "affiliations")]
pub struct Affiliations {
    /// The node name this request pertains to.
    #[xml(attribute)]
    pub node: NodeName,

    /// The actual list of affiliation elements.
    #[xml(child(n = ..))]
    pub affiliations: Vec<Affiliation>,
}

/// An affiliation element.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::PUBSUB_OWNER, name = "affiliation")]
pub struct Affiliation {
    /// The node this affiliation pertains to.
    #[xml(attribute)]
    jid: Jid,

    /// The affiliation you currently have on this node.
    #[xml(attribute)]
    affiliation: AffiliationAttribute,
}

/// Request to configure a node.
#[derive(FromXml, AsXml, Debug, PartialEq, Clone)]
#[xml(namespace = ns::PUBSUB_OWNER, name = "configure")]
pub struct Configure {
    /// The node to be configured.
    #[xml(attribute(default))]
    pub node: Option<NodeName>,

    /// The form to configure it.
    #[xml(child(default))]
    pub form: Option<DataForm>,
}

/// Request to retrieve default configuration.
#[derive(FromXml, AsXml, Debug, PartialEq, Clone)]
#[xml(namespace = ns::PUBSUB_OWNER, name = "default")]
pub struct Default {
    /// The form to configure it.
    #[xml(child(default))]
    pub form: Option<DataForm>,
}

/// Request to delete a node.
#[derive(FromXml, AsXml, Debug, PartialEq, Clone)]
#[xml(namespace = ns::PUBSUB_OWNER, name = "delete")]
pub struct Delete {
    /// The node to be deleted.
    #[xml(attribute)]
    pub node: NodeName,

    /// Redirection to replace the deleted node.
    #[xml(child(default))]
    pub redirect: Option<Redirect>,
}

/// A redirect element.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::PUBSUB_OWNER, name = "redirect")]
pub struct Redirect {
    /// The node this node will be redirected to.
    #[xml(attribute)]
    pub uri: String,
}

/// Request to clear a node.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::PUBSUB_OWNER, name = "purge")]
pub struct Purge {
    /// The node to be cleared.
    #[xml(attribute)]
    pub node: NodeName,
}

/// A request for current subscriptions.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::PUBSUB_OWNER, name = "subscriptions")]
pub struct Subscriptions {
    /// The node to query.
    #[xml(attribute)]
    pub node: NodeName,

    /// The list of subscription elements returned.
    #[xml(child(n = ..))]
    pub subscriptions: Vec<SubscriptionElem>,
}

/// A subscription element, describing the state of a subscription.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::PUBSUB_OWNER, name = "subscription")]
pub struct SubscriptionElem {
    /// The JID affected by this subscription.
    #[xml(attribute)]
    pub jid: Jid,

    /// The state of the subscription.
    #[xml(attribute)]
    pub subscription: Subscription,

    /// Subscription unique id.
    #[xml(attribute(default))]
    pub subid: Option<String>,
}

/// Main payload used to communicate with a PubSubOwner service.
///
/// `<pubsub xmlns="http://jabber.org/protocol/pubsub#owner"/>`
#[derive(Debug, Clone, PartialEq)]
pub enum PubSubOwner {
    /// Manage the affiliations of a node.
    Affiliations(Affiliations),
    /// Request to configure a node, with optional suggested name and suggested configuration.
    Configure(Configure),
    /// Request the default node configuration.
    Default(Default),
    /// Delete a node.
    Delete(Delete),
    /// Purge all items from node.
    Purge(Purge),
    /// Request subscriptions of a node.
    Subscriptions(Subscriptions),
}

impl IqGetPayload for PubSubOwner {}
impl IqSetPayload for PubSubOwner {}
impl IqResultPayload for PubSubOwner {}

impl TryFrom<Element> for PubSubOwner {
    type Error = FromElementError;

    fn try_from(elem: Element) -> Result<PubSubOwner, FromElementError> {
        check_self!(elem, "pubsub", PUBSUB_OWNER);
        check_no_attributes!(elem, "pubsub");

        let mut payload = None;
        for child in elem.children() {
            if child.is("configure", ns::PUBSUB_OWNER) {
                if payload.is_some() {
                    return Err(Error::Other(
                        "Payload is already defined in pubsub owner element.",
                    )
                    .into());
                }
                let configure = Configure::try_from(child.clone())?;
                payload = Some(PubSubOwner::Configure(configure));
            } else {
                return Err(Error::Other("Unknown child in pubsub element.").into());
            }
        }
        payload.ok_or(Error::Other("No payload in pubsub element.").into())
    }
}

impl From<PubSubOwner> for Element {
    fn from(pubsub: PubSubOwner) -> Element {
        Element::builder("pubsub", ns::PUBSUB_OWNER)
            .append_all(match pubsub {
                PubSubOwner::Affiliations(affiliations) => vec![Element::from(affiliations)],
                PubSubOwner::Configure(configure) => vec![Element::from(configure)],
                PubSubOwner::Default(default) => vec![Element::from(default)],
                PubSubOwner::Delete(delete) => vec![Element::from(delete)],
                PubSubOwner::Purge(purge) => vec![Element::from(purge)],
                PubSubOwner::Subscriptions(subscriptions) => vec![Element::from(subscriptions)],
            })
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_forms::{DataFormType, Field, FieldType};
    use core::str::FromStr;
    use jid::BareJid;

    #[test]
    fn affiliations() {
        let elem: Element = "<pubsub xmlns='http://jabber.org/protocol/pubsub#owner'><affiliations node='foo'><affiliation jid='hamlet@denmark.lit' affiliation='owner'/><affiliation jid='polonius@denmark.lit' affiliation='outcast'/></affiliations></pubsub>"
        .parse()
        .unwrap();
        let elem1 = elem.clone();

        let pubsub = PubSubOwner::Affiliations(Affiliations {
            node: NodeName(String::from("foo")),
            affiliations: vec![
                Affiliation {
                    jid: Jid::from(BareJid::from_str("hamlet@denmark.lit").unwrap()),
                    affiliation: AffiliationAttribute::Owner,
                },
                Affiliation {
                    jid: Jid::from(BareJid::from_str("polonius@denmark.lit").unwrap()),
                    affiliation: AffiliationAttribute::Outcast,
                },
            ],
        });

        let elem2 = Element::from(pubsub);
        assert_eq!(elem1, elem2);
    }

    #[test]
    fn configure() {
        let elem: Element = "<pubsub xmlns='http://jabber.org/protocol/pubsub#owner'><configure node='foo'><x xmlns='jabber:x:data' type='submit'><field var='FORM_TYPE' type='hidden'><value>http://jabber.org/protocol/pubsub#node_config</value></field><field var='pubsub#access_model' type='list-single'><value>whitelist</value></field></x></configure></pubsub>"
        .parse()
        .unwrap();
        let elem1 = elem.clone();

        let pubsub = PubSubOwner::Configure(Configure {
            node: Some(NodeName(String::from("foo"))),
            form: Some(DataForm::new(
                DataFormType::Submit,
                ns::PUBSUB_CONFIGURE,
                vec![Field::new("pubsub#access_model", FieldType::ListSingle)
                    .with_value("whitelist")],
            )),
        });

        let elem2 = Element::from(pubsub);
        assert_eq!(elem1, elem2);
    }

    #[test]
    fn test_serialize_configure() {
        let reference: Element = "<pubsub xmlns='http://jabber.org/protocol/pubsub#owner'><configure node='foo'><x xmlns='jabber:x:data' type='submit'/></configure></pubsub>"
        .parse()
        .unwrap();

        let elem: Element = "<x xmlns='jabber:x:data' type='submit'/>".parse().unwrap();

        let form = DataForm::try_from(elem).unwrap();

        let configure = PubSubOwner::Configure(Configure {
            node: Some(NodeName(String::from("foo"))),
            form: Some(form),
        });
        let serialized: Element = configure.into();
        assert_eq!(serialized, reference);
    }

    #[test]
    fn default() {
        let elem: Element = "<pubsub xmlns='http://jabber.org/protocol/pubsub#owner'><default><x xmlns='jabber:x:data' type='submit'><field var='FORM_TYPE' type='hidden'><value>http://jabber.org/protocol/pubsub#node_config</value></field><field var='pubsub#access_model' type='list-single'><value>whitelist</value></field></x></default></pubsub>"
        .parse()
        .unwrap();
        let elem1 = elem.clone();

        let pubsub = PubSubOwner::Default(Default {
            form: Some(DataForm::new(
                DataFormType::Submit,
                ns::PUBSUB_CONFIGURE,
                vec![Field::new("pubsub#access_model", FieldType::ListSingle)
                    .with_value("whitelist")],
            )),
        });

        let elem2 = Element::from(pubsub);
        assert_eq!(elem1, elem2);
    }

    #[test]
    fn delete() {
        let elem: Element = "<pubsub xmlns='http://jabber.org/protocol/pubsub#owner'><delete node='foo'><redirect uri='xmpp:hamlet@denmark.lit?;node=blog'/></delete></pubsub>"
        .parse()
        .unwrap();
        let elem1 = elem.clone();

        let pubsub = PubSubOwner::Delete(Delete {
            node: NodeName(String::from("foo")),
            redirect: Some(Redirect {
                uri: String::from("xmpp:hamlet@denmark.lit?;node=blog"),
            }),
        });

        let elem2 = Element::from(pubsub);
        assert_eq!(elem1, elem2);
    }

    #[test]
    fn purge() {
        let elem: Element = "<pubsub xmlns='http://jabber.org/protocol/pubsub#owner'><purge node='foo'></purge></pubsub>"
        .parse()
        .unwrap();
        let elem1 = elem.clone();

        let pubsub = PubSubOwner::Purge(Purge {
            node: NodeName(String::from("foo")),
        });

        let elem2 = Element::from(pubsub);
        assert_eq!(elem1, elem2);
    }

    #[test]
    fn subscriptions() {
        let elem: Element = "<pubsub xmlns='http://jabber.org/protocol/pubsub#owner'><subscriptions node='foo'><subscription jid='hamlet@denmark.lit' subscription='subscribed'/><subscription jid='polonius@denmark.lit' subscription='unconfigured'/><subscription jid='bernardo@denmark.lit' subscription='subscribed' subid='123-abc'/><subscription jid='bernardo@denmark.lit' subscription='subscribed' subid='004-yyy'/></subscriptions></pubsub>"
        .parse()
        .unwrap();
        let elem1 = elem.clone();

        let pubsub = PubSubOwner::Subscriptions(Subscriptions {
            node: NodeName(String::from("foo")),
            subscriptions: vec![
                SubscriptionElem {
                    jid: Jid::from(BareJid::from_str("hamlet@denmark.lit").unwrap()),
                    subscription: Subscription::Subscribed,
                    subid: None,
                },
                SubscriptionElem {
                    jid: Jid::from(BareJid::from_str("polonius@denmark.lit").unwrap()),
                    subscription: Subscription::Unconfigured,
                    subid: None,
                },
                SubscriptionElem {
                    jid: Jid::from(BareJid::from_str("bernardo@denmark.lit").unwrap()),
                    subscription: Subscription::Subscribed,
                    subid: Some(String::from("123-abc")),
                },
                SubscriptionElem {
                    jid: Jid::from(BareJid::from_str("bernardo@denmark.lit").unwrap()),
                    subscription: Subscription::Subscribed,
                    subid: Some(String::from("004-yyy")),
                },
            ],
        });

        let elem2 = Element::from(pubsub);
        assert_eq!(elem1, elem2);
    }
}
