// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use tokio_xmpp::{jid::Jid, parsers::message::Message};

use crate::{delay::StanzaTimeInfo, Agent, Event};

pub async fn handle_message_group_chat(
    agent: &mut Agent,
    events: &mut Vec<Event>,
    from: Jid,
    message: &Message,
    time_info: StanzaTimeInfo,
) {
    let langs: Vec<&str> = agent.lang.iter().map(String::as_str).collect();

    if let Some((_lang, subject)) = message.get_best_subject(langs.clone()) {
        events.push(Event::RoomSubject(
            from.to_bare(),
            from.resource().map(Into::into),
            subject.0.clone(),
            time_info.clone(),
        ));
    }

    if let Some((_lang, body)) = message.get_best_body(langs) {
        let event = match from.clone().try_into_full() {
            Ok(full) => Event::RoomMessage(
                message.id.clone(),
                from.to_bare(),
                full.resource().into(),
                body.clone(),
                time_info,
            ),
            Err(bare) => Event::ServiceMessage(message.id.clone(), bare, body.clone(), time_info),
        };
        events.push(event)
    }
}
