// Copyright (c) 2022 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use xso::{text::EmptyAsNone, AsXml, FromXml};

use crate::ns;
use crate::Element;
use xso::error::{Error, FromElementError};

generate_attribute!(
    /// Events for real-time text.
    Event, "event", {
        /// Begin a new real-time message.
        New => "new",

        /// Re-initialize the real-time message.
        Reset => "reset",

        /// Modify existing real-time message.
        Edit => "edit",

        /// Signals activation of real-time text.
        Init => "init",

        /// Signals deactivation of real-time text.
        Cancel => "cancel",
    }, Default = Edit
);

/// Supports the transmission of text, including key presses, and text block inserts.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::RTT, name = "t")]
pub struct Insert {
    /// Position in the message to start inserting from.  If None, this means to start from the
    /// end of the message.
    #[xml(attribute(default, name = "p"))]
    pub pos: Option<u32>,

    /// Text to insert.
    #[xml(text = EmptyAsNone)]
    pub text: Option<String>,
}

impl TryFrom<Action> for Insert {
    type Error = Error;

    fn try_from(action: Action) -> Result<Insert, Error> {
        match action {
            Action::Insert(insert) => Ok(insert),
            _ => Err(Error::Other("This is not an insert action.")),
        }
    }
}

// TODO: add a way in the macro to set a default value.
/*
generate_element!(
    Erase, "e", RTT,
    attributes: [
        pos: Option<u32> = "p",
        num: Default<u32> = "n",
    ]
);
*/

/// Supports the behavior of backspace key presses.  Text is removed towards beginning of the
/// message.  This element is also used for all delete operations, including the backspace key, the
/// delete key, and text block deletes.
#[derive(Debug, Clone, PartialEq)]
pub struct Erase {
    /// Position in the message to start erasing from.  If None, this means to start from the end
    /// of the message.
    pub pos: Option<u32>,

    /// Amount of characters to erase, to the left.
    pub num: u32,
}

impl TryFrom<Element> for Erase {
    type Error = FromElementError;
    fn try_from(elem: Element) -> Result<Erase, FromElementError> {
        check_self!(elem, "e", RTT);
        check_no_unknown_attributes!(elem, "e", ["p", "n"]);
        let pos = get_attr!(elem, "p", Option);
        let num = get_attr!(elem, "n", Option).unwrap_or(1);
        check_no_children!(elem, "e");
        Ok(Erase { pos, num })
    }
}

impl From<Erase> for Element {
    fn from(elem: Erase) -> Element {
        Element::builder("e", ns::RTT)
            .attr("p", elem.pos)
            .attr("n", elem.num)
            .build()
    }
}

impl TryFrom<Action> for Erase {
    type Error = Error;

    fn try_from(action: Action) -> Result<Erase, Error> {
        match action {
            Action::Erase(erase) => Ok(erase),
            _ => Err(Error::Other("This is not an erase action.")),
        }
    }
}

/// Allow for the transmission of intervals, between real-time text actions, to recreate the
/// pauses between key presses.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::RTT, name = "w")]
pub struct Wait {
    /// Amount of milliseconds to wait before the next action.
    #[xml(attribute = "n")]
    pub time: u32,
}

impl TryFrom<Action> for Wait {
    type Error = Error;

    fn try_from(action: Action) -> Result<Wait, Error> {
        match action {
            Action::Wait(wait) => Ok(wait),
            _ => Err(Error::Other("This is not a wait action.")),
        }
    }
}

/// Choice between the three possible actions.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Insert text action.
    Insert(Insert),

    /// Erase text action.
    Erase(Erase),

    /// Wait action.
    Wait(Wait),
}

impl TryFrom<Element> for Action {
    type Error = FromElementError;

    fn try_from(elem: Element) -> Result<Action, FromElementError> {
        match elem.name() {
            "t" => Insert::try_from(elem).map(Action::Insert),
            "e" => Erase::try_from(elem).map(Action::Erase),
            "w" => Wait::try_from(elem).map(Action::Wait),
            _ => Err(FromElementError::Mismatch(elem)),
        }
    }
}

impl From<Action> for Element {
    fn from(action: Action) -> Element {
        match action {
            Action::Insert(insert) => Element::from(insert),
            Action::Erase(erase) => Element::from(erase),
            Action::Wait(wait) => Element::from(wait),
        }
    }
}

// TODO: Allow a wildcard name to the macro, to simplify the following code:
/*
generate_element!(
    Rtt, "rtt", RTT,
    attributes: [
        seq: Required<u32> = "seq",
        event: Default<Event> = "event",
        id: Option<String> = "id",
    ],
    children: [
        actions: Vec<Action> = (*, RTT) => Action,
    ]
);
*/

/// Element transmitted at regular interval by the sender client while a message is being composed.
#[derive(Debug, Clone, PartialEq)]
pub struct Rtt {
    /// Counter to maintain synchronisation of real-time text.  Senders MUST increment this value
    /// by 1 for each subsequent edit to the same real-time message, including when appending new
    /// text.  Receiving clients MUST monitor this 'seq' value as a lightweight verification on the
    /// synchronization of real-time text messages.  The bounds of 'seq' is 31-bits, the range of
    /// positive values for a signed 32-bit integer.
    pub seq: u32,

    /// This attribute signals events for real-time text.
    pub event: Event,

    /// When editing a message using XEP-0308, this references the id of the message being edited.
    pub id: Option<String>,

    /// Vector of actions being transmitted by this element.
    pub actions: Vec<Action>,
}

impl TryFrom<Element> for Rtt {
    type Error = FromElementError;
    fn try_from(elem: Element) -> Result<Rtt, FromElementError> {
        check_self!(elem, "rtt", RTT);

        check_no_unknown_attributes!(elem, "rtt", ["seq", "event", "id"]);
        let seq = get_attr!(elem, "seq", Required);
        let event = get_attr!(elem, "event", Default);
        let id = get_attr!(elem, "id", Option);

        let mut actions = Vec::new();
        for child in elem.children() {
            if child.ns() != ns::RTT {
                return Err(Error::Other("Unknown child in rtt element.").into());
            }
            actions.push(Action::try_from(child.clone())?);
        }

        Ok(Rtt {
            seq,
            event,
            id,
            actions,
        })
    }
}

impl From<Rtt> for Element {
    fn from(elem: Rtt) -> Element {
        Element::builder("rtt", ns::RTT)
            .attr("seq", elem.seq)
            .attr("event", elem.event)
            .attr("id", elem.id)
            .append_all(elem.actions)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn test_size() {
        assert_size!(Event, 1);
        assert_size!(Insert, 20);
        assert_size!(Erase, 12);
        assert_size!(Wait, 4);
        assert_size!(Action, 20);
        assert_size!(Rtt, 32);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_size() {
        assert_size!(Event, 1);
        assert_size!(Insert, 32);
        assert_size!(Erase, 12);
        assert_size!(Wait, 4);
        assert_size!(Action, 32);
        assert_size!(Rtt, 56);
    }

    #[test]
    fn simple() {
        let elem: Element = "<rtt xmlns='urn:xmpp:rtt:0' seq='0'/>".parse().unwrap();
        let rtt = Rtt::try_from(elem).unwrap();
        assert_eq!(rtt.seq, 0);
        assert_eq!(rtt.event, Event::Edit);
        assert_eq!(rtt.id, None);
        assert_eq!(rtt.actions.len(), 0);
    }

    #[test]
    fn sequence() {
        let elem: Element = "<rtt xmlns='urn:xmpp:rtt:0' seq='0' event='new'><t>Hello,</t><w n='50'/><e/><t>!</t></rtt>"
            .parse()
            .unwrap();

        let rtt = Rtt::try_from(elem).unwrap();
        assert_eq!(rtt.seq, 0);
        assert_eq!(rtt.event, Event::New);
        assert_eq!(rtt.id, None);

        let mut actions = rtt.actions.into_iter();
        assert_eq!(actions.len(), 4);

        let t: Insert = actions.next().unwrap().try_into().unwrap();
        assert_eq!(t.pos, None);
        assert_eq!(t.text, Some(String::from("Hello,")));

        let w: Wait = actions.next().unwrap().try_into().unwrap();
        assert_eq!(w.time, 50);

        let e: Erase = actions.next().unwrap().try_into().unwrap();
        assert_eq!(e.pos, None);
        assert_eq!(e.num, 1);

        let t: Insert = actions.next().unwrap().try_into().unwrap();
        assert_eq!(t.pos, None);
        assert_eq!(t.text, Some(String::from("!")));
    }
}
