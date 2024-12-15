// Copyright (c) 2019 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//!
//! Chatroom bookmarks from [XEP-0402](https://xmpp.org/extensions/xep-0402.html) for newer servers
//! which advertise `urn:xmpp:bookmarks:1#compat` on the user's BareJID in a disco info request.
//! On legacy non-compliant servers, use the [`private`][crate::private] module instead.
//!
//! See [ModernXMPP docs](https://docs.modernxmpp.org/client/groupchat/#bookmarks) on how to handle all historic
//! and newer specifications for your clients.
//!
//! This module exposes the [`Autojoin`][crate::bookmarks2::Autojoin] boolean flag, the [`Conference`][crate::bookmarks2::Conference] chatroom element, and the [BOOKMARKS2][crate::ns::BOOKMARKS2] XML namespace.

use xso::{AsXml, FromXml};

use crate::ns;
use minidom::Element;

generate_attribute!(
    /// Whether a conference bookmark should be joined automatically.
    Autojoin,
    "autojoin",
    bool
);

/// Potential extensions in a conference.
#[derive(FromXml, AsXml, Debug, Clone, Default)]
#[xml(namespace = ns::BOOKMARKS2, name = "extensions")]
pub struct Extensions {
    /// Extension elements.
    #[xml(element(n = ..))]
    pub payloads: Vec<Element>,
}

/// A conference bookmark.
#[derive(FromXml, AsXml, Debug, Clone, Default)]
#[xml(namespace = ns::BOOKMARKS2, name = "conference")]
pub struct Conference {
    /// Whether a conference bookmark should be joined automatically.
    #[xml(attribute(default))]
    pub autojoin: Autojoin,

    /// A user-defined name for this conference.
    #[xml(attribute(default))]
    pub name: Option<String>,

    /// The nick the user will use to join this conference.
    #[xml(extract(default, fields(text(type_ = String))))]
    pub nick: Option<String>,

    /// The password required to join this conference.
    #[xml(extract(default, fields(text(type_ = String))))]
    pub password: Option<String>,

    /// Extension elements.
    #[xml(child(default))]
    pub extensions: Option<Extensions>,
}

impl Conference {
    /// Create a new conference.
    pub fn new() -> Conference {
        Conference::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pubsub::{pubsub::Item as PubSubItem, PubSubEvent};

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn test_size() {
        assert_size!(Conference, 52);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_size() {
        assert_size!(Conference, 104);
    }

    #[test]
    fn simple() {
        let elem: Element = "<conference xmlns='urn:xmpp:bookmarks:1'/>"
            .parse()
            .unwrap();
        let elem1 = elem.clone();
        let conference = Conference::try_from(elem).unwrap();
        assert_eq!(conference.autojoin, Autojoin::False);
        assert_eq!(conference.name, None);
        assert_eq!(conference.nick, None);
        assert_eq!(conference.password, None);

        let elem2 = Element::from(Conference::new());
        assert_eq!(elem1, elem2);
    }

    #[test]
    fn complete() {
        let elem: Element = "<conference xmlns='urn:xmpp:bookmarks:1' autojoin='true' name='Test MUC'><nick>Coucou</nick><password>secret</password><extensions><test xmlns='urn:xmpp:unknown' /></extensions></conference>".parse().unwrap();
        let conference = Conference::try_from(elem).unwrap();
        assert_eq!(conference.autojoin, Autojoin::True);
        assert_eq!(conference.name, Some(String::from("Test MUC")));
        assert_eq!(conference.clone().nick.unwrap(), "Coucou");
        assert_eq!(conference.clone().password.unwrap(), "secret");
        let payloads = conference.clone().extensions.unwrap().payloads;
        assert_eq!(payloads.len(), 1);
        assert!(payloads[0].is("test", "urn:xmpp:unknown"));
    }

    #[test]
    fn wrapped() {
        let elem: Element = "<item xmlns='http://jabber.org/protocol/pubsub' id='test-muc@muc.localhost'><conference xmlns='urn:xmpp:bookmarks:1' autojoin='true' name='Test MUC'><nick>Coucou</nick><password>secret</password></conference></item>".parse().unwrap();
        let item = PubSubItem::try_from(elem).unwrap();
        let payload = item.payload.clone().unwrap();
        println!("FOO: payload: {:?}", payload);
        // let conference = Conference::try_from(payload).unwrap();
        let conference = Conference::try_from(payload).unwrap();
        println!("FOO: conference: {:?}", conference);
        assert_eq!(conference.autojoin, Autojoin::True);
        assert_eq!(conference.name, Some(String::from("Test MUC")));
        assert_eq!(conference.clone().nick.unwrap(), "Coucou");
        assert_eq!(conference.clone().password.unwrap(), "secret");

        let elem: Element = "<event xmlns='http://jabber.org/protocol/pubsub#event'><items node='urn:xmpp:bookmarks:1'><item xmlns='http://jabber.org/protocol/pubsub#event' id='test-muc@muc.localhost'><conference xmlns='urn:xmpp:bookmarks:1' autojoin='true' name='Test MUC'><nick>Coucou</nick><password>secret</password></conference></item></items></event>".parse().unwrap();
        let mut items = match PubSubEvent::try_from(elem) {
            Ok(PubSubEvent::PublishedItems { node, items }) => {
                assert_eq!(&node.0, ns::BOOKMARKS2);
                items
            }
            _ => panic!(),
        };
        assert_eq!(items.len(), 1);
        let item = items.pop().unwrap();
        let payload = item.payload.clone().unwrap();
        let conference = Conference::try_from(payload).unwrap();
        assert_eq!(conference.autojoin, Autojoin::True);
        assert_eq!(conference.name, Some(String::from("Test MUC")));
        assert_eq!(conference.clone().nick.unwrap(), "Coucou");
        assert_eq!(conference.clone().password.unwrap(), "secret");
    }
}
