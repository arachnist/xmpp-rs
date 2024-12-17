// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use rand::{thread_rng, Rng};
use xmpp_parsers::{iq::Iq, jid::Jid, message::Message, presence::Presence};
use xso::{AsXml, FromXml};

use crate::xmlstream::XmppStreamElement;
use crate::Error;

fn make_id() -> String {
    let id: u64 = thread_rng().gen();
    format!("{}", id)
}

/// A stanza sent/received over the stream.
#[derive(FromXml, AsXml, Debug)]
#[xml()]
pub enum Stanza {
    /// IQ stanza
    #[xml(transparent)]
    Iq(Iq),

    /// Message stanza
    #[xml(transparent)]
    Message(Message),

    /// Presence stanza
    #[xml(transparent)]
    Presence(Presence),
}

impl Stanza {
    /// Assign a random ID to the stanza, if no ID has been assigned yet.
    pub fn ensure_id(&mut self) -> &str {
        match self {
            Self::Iq(iq) => {
                if iq.id.len() == 0 {
                    iq.id = make_id();
                }
                &iq.id
            }
            Self::Message(message) => message.id.get_or_insert_with(make_id),
            Self::Presence(presence) => presence.id.get_or_insert_with(make_id),
        }
    }
}

impl From<Iq> for Stanza {
    fn from(other: Iq) -> Self {
        Self::Iq(other)
    }
}

impl From<Presence> for Stanza {
    fn from(other: Presence) -> Self {
        Self::Presence(other)
    }
}

impl From<Message> for Stanza {
    fn from(other: Message) -> Self {
        Self::Message(other)
    }
}

impl TryFrom<Stanza> for Message {
    type Error = Stanza;

    fn try_from(other: Stanza) -> Result<Self, Self::Error> {
        match other {
            Stanza::Message(st) => Ok(st),
            other => Err(other),
        }
    }
}

impl TryFrom<Stanza> for Presence {
    type Error = Stanza;

    fn try_from(other: Stanza) -> Result<Self, Self::Error> {
        match other {
            Stanza::Presence(st) => Ok(st),
            other => Err(other),
        }
    }
}

impl TryFrom<Stanza> for Iq {
    type Error = Stanza;

    fn try_from(other: Stanza) -> Result<Self, Self::Error> {
        match other {
            Stanza::Iq(st) => Ok(st),
            other => Err(other),
        }
    }
}

impl From<Stanza> for XmppStreamElement {
    fn from(other: Stanza) -> Self {
        match other {
            Stanza::Iq(st) => Self::Iq(st),
            Stanza::Message(st) => Self::Message(st),
            Stanza::Presence(st) => Self::Presence(st),
        }
    }
}

/// High-level event on the Stream implemented by Client and Component
#[derive(Debug)]
pub enum Event {
    /// Stream is connected and initialized
    Online {
        /// Server-set Jabber-Id for your session
        ///
        /// This may turn out to be a different JID resource than
        /// expected, so use this one instead of the JID with which
        /// the connection was setup.
        bound_jid: Jid,
        /// Was this session resumed?
        ///
        /// Not yet implemented for the Client
        resumed: bool,
    },
    /// Stream end
    Disconnected(Error),
    /// Received stanza/nonza
    Stanza(Stanza),
}

impl Event {
    /// `Online` event?
    pub fn is_online(&self) -> bool {
        match *self {
            Event::Online { .. } => true,
            _ => false,
        }
    }

    /// Get the server-assigned JID for the `Online` event
    pub fn get_jid(&self) -> Option<&Jid> {
        match *self {
            Event::Online { ref bound_jid, .. } => Some(bound_jid),
            _ => None,
        }
    }

    /// If this is a `Stanza` event, get its data
    pub fn as_stanza(&self) -> Option<&Stanza> {
        match *self {
            Event::Stanza(ref stanza) => Some(stanza),
            _ => None,
        }
    }

    /// If this is a `Stanza` event, unwrap into its data
    pub fn into_stanza(self) -> Option<Stanza> {
        match self {
            Event::Stanza(stanza) => Some(stanza),
            _ => None,
        }
    }
}
