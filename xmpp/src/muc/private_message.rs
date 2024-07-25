// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use tokio_xmpp::connect::ServerConnector;
use tokio_xmpp::{
    jid::{BareJid, Jid},
    parsers::{
        message::{Body, Message, MessageType},
        muc::user::MucUser,
    },
};

use crate::{Agent, RoomNick};

pub async fn send_room_private_message<C: ServerConnector>(
    agent: &mut Agent<C>,
    room: BareJid,
    recipient: RoomNick,
    lang: &str,
    text: &str,
) {
    let recipient: Jid = room.with_resource_str(&recipient).unwrap().into();
    let mut message = Message::new(recipient).with_payload(MucUser::new());
    message.type_ = MessageType::Chat;
    message
        .bodies
        .insert(String::from(lang), Body(String::from(text)));
    let _ = agent.client.send_stanza(message.into()).await;
}
