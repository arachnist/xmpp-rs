// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use xso::{AsXml, FromXml};

use xmpp_parsers::{component, iq::Iq, message::Message, presence::Presence, sasl, starttls};

/// Any valid XMPP stream-level element.
#[derive(FromXml, AsXml, Debug)]
#[xml()]
pub enum XmppStreamElement {
    /// IQ stanza
    #[xml(transparent)]
    Iq(Iq),

    /// Message stanza
    #[xml(transparent)]
    Message(Message),

    /// Presence stanza
    #[xml(transparent)]
    Presence(Presence),

    /// SASL-related nonza
    #[xml(transparent)]
    Sasl(sasl::Nonza),

    /// STARTTLS-related nonza
    #[xml(transparent)]
    Starttls(starttls::Nonza),

    /// Component protocol nonzas
    #[xml(transparent)]
    ComponentHandshake(component::Handshake),
}
