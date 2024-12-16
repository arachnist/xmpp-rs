// Copyright (c) 2019 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::Agent;
use crate::{
    muc::room::{JoinRoomSettings, LeaveRoomSettings},
    Event,
};
use std::str::FromStr;
use tokio_xmpp::{
    connect::ServerConnector,
    jid::{BareJid, Jid},
    minidom::Element,
    parsers::{
        bookmarks2, ns,
        pubsub::{event::PubSubEvent, pubsub::PubSub},
    },
};

#[cfg(feature = "avatars")]
pub(crate) mod avatar;

pub(crate) async fn handle_event<C: ServerConnector>(
    #[cfg_attr(not(feature = "avatars"), allow(unused_variables))] from: &Jid,
    elem: Element,
    #[cfg_attr(not(feature = "avatars"), allow(unused_variables))] agent: &mut Agent<C>,
) -> Vec<Event> {
    // We allow the useless mut warning for no-default-features,
    // since for now only avatars pushes events here.
    #[allow(unused_mut)]
    let mut events = Vec::new();

    let event = PubSubEvent::try_from(elem);
    trace!("PubSub event: {:#?}", event);
    match event {
        Ok(PubSubEvent::PublishedItems { node, items }) => {
            match node.0 {
                #[cfg(feature = "avatars")]
                ref node if node == ns::AVATAR_METADATA => {
                    let new_events =
                        avatar::handle_metadata_pubsub_event(&from, agent, items).await;
                    events.extend(new_events);
                }
                ref node if node == ns::BOOKMARKS2 => {
                    // TODO: Check that our bare JID is the sender.
                    assert_eq!(items.len(), 1);
                    let item = items.clone().pop().unwrap();
                    let jid = BareJid::from_str(&item.id.clone().unwrap().0).unwrap();
                    let payload = item.payload.clone().unwrap();
                    match bookmarks2::Conference::try_from(payload) {
                        Ok(conference) => {
                            if conference.autojoin {
                                if !agent.rooms_joined.contains_key(&jid) {
                                    agent
                                        .join_room(JoinRoomSettings {
                                            room: jid,
                                            nick: conference.nick,
                                            password: conference.password,
                                            status: None,
                                        })
                                        .await;
                                }
                            } else {
                                // So maybe another client of ours left the room... let's leave it too
                                agent.leave_room(LeaveRoomSettings::new(jid)).await;
                            }
                        }
                        Err(err) => println!("not bookmark: {}", err),
                    }
                }
                ref node => unimplemented!("node {}", node),
            }
        }
        Ok(PubSubEvent::RetractedItems { node, items }) => {
            match node.0 {
                ref node if node == ns::BOOKMARKS2 => {
                    // TODO: Check that our bare JID is the sender.
                    assert_eq!(items.len(), 1);
                    let item = items.clone().pop().unwrap();
                    let jid = BareJid::from_str(&item.0).unwrap();

                    agent.leave_room(LeaveRoomSettings::new(jid)).await;
                }
                ref node => unimplemented!("node {}", node),
            }
        }
        Ok(PubSubEvent::Purge { node }) => match node.0 {
            ref node if node == ns::BOOKMARKS2 => {
                warn!("The bookmarks2 PEP node was deleted!");
            }
            ref node => unimplemented!("node {}", node),
        },
        Err(e) => {
            error!("Error parsing PubSub event: {}", e);
        }
        _ => unimplemented!("PubSub event: {:#?}", event),
    }
    events
}

pub(crate) async fn handle_iq_result<C: ServerConnector>(
    #[cfg_attr(not(feature = "avatars"), allow(unused_variables))] from: &Jid,
    elem: Element,
    agent: &mut Agent<C>,
) -> impl IntoIterator<Item = Event> {
    // We allow the useless mut warning for no-default-features,
    // since for now only avatars pushes events here.
    #[allow(unused_mut)]
    let mut events = Vec::new();

    let pubsub = PubSub::try_from(elem).unwrap();
    trace!("PubSub: {:#?}", pubsub);
    if let PubSub::Items(items) = pubsub {
        match items.node.0.clone() {
            #[cfg(feature = "avatars")]
            ref node if node == ns::AVATAR_DATA => {
                let new_events = avatar::handle_data_pubsub_iq(&from, &items);
                events.extend(new_events);
            }
            ref node if node == ns::BOOKMARKS2 => {
                // Keep track of the new added/removed rooms in the bookmarks2 list.
                // The rooms we joined which are no longer in the list should be left ASAP.
                let mut new_room_list: Vec<BareJid> = Vec::new();

                for item in items.items {
                    let item = item.0;
                    let jid = BareJid::from_str(&item.id.clone().unwrap().0).unwrap();
                    let payload = item.payload.clone().unwrap();
                    match bookmarks2::Conference::try_from(payload) {
                        Ok(conference) => {
                            // This room was either marked for join or leave, but it was still in the bookmarks.
                            // Keep track in new_room_list.
                            new_room_list.push(jid.clone());

                            if conference.autojoin {
                                if !agent.rooms_joined.contains_key(&jid) {
                                    agent
                                        .join_room(JoinRoomSettings {
                                            room: jid,
                                            nick: conference.nick,
                                            password: conference.password,
                                            status: None,
                                        })
                                        .await;
                                }
                            } else {
                                // Leave the room that is no longer autojoin
                                agent.leave_room(LeaveRoomSettings::new(jid)).await;
                            }
                        }
                        Err(err) => {
                            warn!("Wrong payload type in bookmarks2 item: {}", err);
                        }
                    }
                }

                // Now we leave the rooms that are no longer in the bookmarks
                let mut rooms_to_leave: Vec<BareJid> = Vec::new();
                for (room, _nick) in &agent.rooms_joined {
                    if !new_room_list.contains(&room) {
                        rooms_to_leave.push(room.clone());
                    }
                }

                for room in rooms_to_leave {
                    agent.leave_room(LeaveRoomSettings::new(room)).await;
                }
            }
            _ => unimplemented!(),
        }
    }
    events
}
