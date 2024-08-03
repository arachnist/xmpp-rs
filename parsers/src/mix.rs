// Copyright (c) 2020 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// TODO: validate nicks by applying the “nickname” profile of the PRECIS OpaqueString class, as
// defined in RFC 7700.

use xso::{AsXml, FromXml};

use crate::iq::{IqResultPayload, IqSetPayload};
use crate::message::MessagePayload;
use crate::ns;
use crate::pubsub::{NodeName, PubSubPayload};
use jid::BareJid;

generate_id!(
    /// The identifier a participant receives when joining a channel.
    ParticipantId
);

impl ParticipantId {
    /// Create a new ParticipantId.
    pub fn new<P: Into<String>>(participant: P) -> ParticipantId {
        ParticipantId(participant.into())
    }
}

generate_id!(
    /// A MIX channel identifier.
    ChannelId
);

/// Represents a participant in a MIX channel, usually returned on the
/// urn:xmpp:mix:nodes:participants PubSub node.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::MIX_CORE, name = "participant")]
pub struct Participant {
    /// The nick of this participant.
    #[xml(extract(namespace = ns::MIX_CORE, name = "nick", fields(text)))]
    pub nick: String,

    /// The bare JID of this participant.
    #[xml(extract(namespace = ns::MIX_CORE, name = "jid", fields(text)))]
    pub jid: BareJid,
}

impl PubSubPayload for Participant {}

impl Participant {
    /// Create a new MIX participant.
    pub fn new<J: Into<BareJid>, N: Into<String>>(jid: J, nick: N) -> Participant {
        Participant {
            nick: nick.into(),
            jid: jid.into(),
        }
    }
}

/// A node to subscribe to.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::MIX_CORE, name = "subscribe")]
pub struct Subscribe {
    /// The PubSub node to subscribe to.
    #[xml(attribute)]
    pub node: NodeName,
}

impl Subscribe {
    /// Create a new Subscribe element.
    pub fn new<N: Into<String>>(node: N) -> Subscribe {
        Subscribe {
            node: NodeName(node.into()),
        }
    }
}

/// A request from a user’s server to join a MIX channel.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::MIX_CORE, name = "join")]
pub struct Join {
    /// The participant identifier returned by the MIX service on successful join.
    #[xml(attribute(default))]
    pub id: Option<ParticipantId>,

    /// The nick requested by the user or set by the service.
    #[xml(extract(namespace = ns::MIX_CORE, name = "nick", fields(text)))]
    pub nick: String,

    /// Which MIX nodes to subscribe to.
    #[xml(child(n = ..))]
    pub subscribes: Vec<Subscribe>,
}

impl IqSetPayload for Join {}
impl IqResultPayload for Join {}

impl Join {
    /// Create a new Join element.
    pub fn from_nick_and_nodes<N: Into<String>>(nick: N, nodes: &[&str]) -> Join {
        let subscribes = nodes.iter().cloned().map(Subscribe::new).collect();
        Join {
            id: None,
            nick: nick.into(),
            subscribes,
        }
    }

    /// Sets the JID on this update-subscription.
    pub fn with_id<I: Into<String>>(mut self, id: I) -> Self {
        self.id = Some(ParticipantId(id.into()));
        self
    }
}

/// Update a given subscription.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::MIX_CORE, name = "update-subscription")]
pub struct UpdateSubscription {
    /// The JID of the user to be affected.
    // TODO: why is it not a participant id instead?
    #[xml(attribute(default))]
    pub jid: Option<BareJid>,

    /// The list of additional nodes to subscribe to.
    // TODO: what happens when we are already subscribed?  Also, how do we unsubscribe from
    // just one?
    #[xml(child(n = ..))]
    pub subscribes: Vec<Subscribe>,
}

impl IqSetPayload for UpdateSubscription {}
impl IqResultPayload for UpdateSubscription {}

impl UpdateSubscription {
    /// Create a new UpdateSubscription element.
    pub fn from_nodes(nodes: &[&str]) -> UpdateSubscription {
        let subscribes = nodes.iter().cloned().map(Subscribe::new).collect();
        UpdateSubscription {
            jid: None,
            subscribes,
        }
    }

    /// Sets the JID on this update-subscription.
    pub fn with_jid(mut self, jid: BareJid) -> Self {
        self.jid = Some(jid);
        self
    }
}

/// Request to leave a given MIX channel.  It will automatically unsubscribe the user from all
/// nodes on this channel.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::MIX_CORE, name = "leave")]
pub struct Leave;

impl IqSetPayload for Leave {}
impl IqResultPayload for Leave {}

/// A request to change the user’s nick.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::MIX_CORE, name = "setnick")]
pub struct SetNick {
    /// The new requested nick.
    #[xml(extract(namespace = ns::MIX_CORE, name = "nick", fields(text)))]
    pub nick: String,
}

impl IqSetPayload for SetNick {}
impl IqResultPayload for SetNick {}

impl SetNick {
    /// Create a new SetNick element.
    pub fn new<N: Into<String>>(nick: N) -> SetNick {
        SetNick { nick: nick.into() }
    }
}

/// Message payload describing who actually sent the message, since unlike in MUC, all messages
/// are sent from the channel’s JID.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::MIX_CORE, name = "mix")]
pub struct Mix {
    /// The nick of the user who said something.
    #[xml(extract(namespace = ns::MIX_CORE, name = "nick", fields(text)))]
    pub nick: String,

    /// The JID of the user who said something.
    #[xml(extract(namespace = ns::MIX_CORE, name = "jid", fields(text)))]
    pub jid: BareJid,
}

impl MessagePayload for Mix {}

impl Mix {
    /// Create a new Mix element.
    pub fn new<N: Into<String>, J: Into<BareJid>>(nick: N, jid: J) -> Mix {
        Mix {
            nick: nick.into(),
            jid: jid.into(),
        }
    }
}

/// Create a new MIX channel.
#[derive(FromXml, AsXml, PartialEq, Clone, Debug, Default)]
#[xml(namespace = ns::MIX_CORE, name = "create")]
pub struct Create {
    /// The requested channel identifier.
    #[xml(attribute(default))]
    pub channel: Option<ChannelId>,
}

impl IqSetPayload for Create {}
impl IqResultPayload for Create {}

impl Create {
    /// Create a new ad-hoc Create element.
    pub fn new() -> Create {
        Create::default()
    }

    /// Create a new Create element with a channel identifier.
    pub fn from_channel_id<C: Into<String>>(channel: C) -> Create {
        Create {
            channel: Some(ChannelId(channel.into())),
        }
    }
}

/// Destroy a given MIX channel.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::MIX_CORE, name = "destroy")]
pub struct Destroy {
    /// The channel identifier to be destroyed.
    #[xml(attribute)]
    pub channel: ChannelId,
}

// TODO: section 7.3.4, example 33, doesn’t mirror the <destroy/> in the iq result unlike every
// other section so far.
impl IqSetPayload for Destroy {}

impl Destroy {
    /// Create a new Destroy element.
    pub fn new<C: Into<String>>(channel: C) -> Destroy {
        Destroy {
            channel: ChannelId(channel.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minidom::Element;

    #[test]
    fn participant() {
        let elem: Element = "<participant xmlns='urn:xmpp:mix:core:1'><jid>foo@bar</jid><nick>coucou</nick></participant>"
            .parse()
            .unwrap();
        let participant = Participant::try_from(elem).unwrap();
        assert_eq!(participant.nick, "coucou");
        assert_eq!(participant.jid.as_str(), "foo@bar");
    }

    #[test]
    fn join() {
        let elem: Element = "<join xmlns='urn:xmpp:mix:core:1'><subscribe node='urn:xmpp:mix:nodes:messages'/><subscribe node='urn:xmpp:mix:nodes:info'/><nick>coucou</nick></join>"
            .parse()
            .unwrap();
        let join = Join::try_from(elem).unwrap();
        assert_eq!(join.nick, "coucou");
        assert_eq!(join.id, None);
        assert_eq!(join.subscribes.len(), 2);
        assert_eq!(join.subscribes[0].node.0, "urn:xmpp:mix:nodes:messages");
        assert_eq!(join.subscribes[1].node.0, "urn:xmpp:mix:nodes:info");
    }

    #[test]
    fn update_subscription() {
        let elem: Element = "<update-subscription xmlns='urn:xmpp:mix:core:1'><subscribe node='urn:xmpp:mix:nodes:participants'/></update-subscription>"
            .parse()
            .unwrap();
        let update_subscription = UpdateSubscription::try_from(elem).unwrap();
        assert_eq!(update_subscription.jid, None);
        assert_eq!(update_subscription.subscribes.len(), 1);
        assert_eq!(
            update_subscription.subscribes[0].node.0,
            "urn:xmpp:mix:nodes:participants"
        );
    }

    #[test]
    fn leave() {
        let elem: Element = "<leave xmlns='urn:xmpp:mix:core:1'/>".parse().unwrap();
        Leave::try_from(elem).unwrap();
    }

    #[test]
    fn setnick() {
        let elem: Element = "<setnick xmlns='urn:xmpp:mix:core:1'><nick>coucou</nick></setnick>"
            .parse()
            .unwrap();
        let setnick = SetNick::try_from(elem).unwrap();
        assert_eq!(setnick.nick, "coucou");
    }

    #[test]
    fn message_mix() {
        let elem: Element =
            "<mix xmlns='urn:xmpp:mix:core:1'><jid>foo@bar</jid><nick>coucou</nick></mix>"
                .parse()
                .unwrap();
        let mix = Mix::try_from(elem).unwrap();
        assert_eq!(mix.nick, "coucou");
        assert_eq!(mix.jid.as_str(), "foo@bar");
    }

    #[test]
    fn create() {
        let elem: Element = "<create xmlns='urn:xmpp:mix:core:1' channel='coucou'/>"
            .parse()
            .unwrap();
        let create = Create::try_from(elem).unwrap();
        assert_eq!(create.channel.unwrap().0, "coucou");

        let elem: Element = "<create xmlns='urn:xmpp:mix:core:1'/>".parse().unwrap();
        let create = Create::try_from(elem).unwrap();
        assert_eq!(create.channel, None);
    }

    #[test]
    fn destroy() {
        let elem: Element = "<destroy xmlns='urn:xmpp:mix:core:1' channel='coucou'/>"
            .parse()
            .unwrap();
        let destroy = Destroy::try_from(elem).unwrap();
        assert_eq!(destroy.channel.0, "coucou");
    }

    #[test]
    fn serialise() {
        let elem: Element = Join::from_nick_and_nodes("coucou", &["foo", "bar"]).into();
        let xml = String::from(&elem);
        assert_eq!(xml, "<join xmlns='urn:xmpp:mix:core:1'><nick>coucou</nick><subscribe node=\"foo\"/><subscribe node=\"bar\"/></join>");

        let elem: Element = UpdateSubscription::from_nodes(&["foo", "bar"]).into();
        let xml = String::from(&elem);
        assert_eq!(xml, "<update-subscription xmlns='urn:xmpp:mix:core:1'><subscribe node=\"foo\"/><subscribe node=\"bar\"/></update-subscription>");

        let elem: Element = Leave.into();
        let xml = String::from(&elem);
        assert_eq!(xml, "<leave xmlns='urn:xmpp:mix:core:1'/>");

        let elem: Element = SetNick::new("coucou").into();
        let xml = String::from(&elem);
        assert_eq!(
            xml,
            "<setnick xmlns='urn:xmpp:mix:core:1'><nick>coucou</nick></setnick>"
        );

        let elem: Element = Mix::new("coucou", "coucou@example".parse::<BareJid>().unwrap()).into();
        let xml = String::from(&elem);
        assert_eq!(
            xml,
            "<mix xmlns='urn:xmpp:mix:core:1'><nick>coucou</nick><jid>coucou@example</jid></mix>"
        );

        let elem: Element = Create::new().into();
        let xml = String::from(&elem);
        assert_eq!(xml, "<create xmlns='urn:xmpp:mix:core:1'/>");

        let elem: Element = Create::from_channel_id("coucou").into();
        let xml = String::from(&elem);
        assert_eq!(
            xml,
            "<create xmlns='urn:xmpp:mix:core:1' channel=\"coucou\"/>"
        );

        let elem: Element = Destroy::new("coucou").into();
        let xml = String::from(&elem);
        assert_eq!(
            xml,
            "<destroy xmlns='urn:xmpp:mix:core:1' channel=\"coucou\"/>"
        );
    }
}
