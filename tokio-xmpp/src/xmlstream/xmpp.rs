// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use xso::{AsXml, FromXml};

use xmpp_parsers::{component, sasl, sm, starttls, stream_error::ReceivedStreamError};

use crate::Stanza;

/// Any valid XMPP stream-level element.
#[derive(FromXml, AsXml, Debug)]
#[xml()]
pub enum XmppStreamElement {
    /// Stanza
    #[xml(transparent)]
    Stanza(Stanza),

    /// SASL-related nonza
    #[xml(transparent)]
    Sasl(sasl::Nonza),

    /// STARTTLS-related nonza
    #[xml(transparent)]
    Starttls(starttls::Nonza),

    /// Component protocol nonzas
    #[xml(transparent)]
    ComponentHandshake(component::Handshake),

    /// Stream error received
    #[xml(transparent)]
    StreamError(ReceivedStreamError),

    /// XEP-0198 nonzas
    #[xml(transparent)]
    SM(sm::Nonza),
}
