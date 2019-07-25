// Copyright (c) 2019 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use futures::prelude::*;
use std::env::args;
use std::process::exit;
use tokio::runtime::current_thread::Runtime;
use xmpp_parsers::message::MessageType;
use xmpp::{ClientBuilder, ClientType, ClientFeature, Event};

fn main() {
    let args: Vec<String> = args().collect();
    if args.len() != 3 {
        println!("Usage: {} <jid> <password>", args[0]);
        exit(1);
    }
    let jid = &args[1];
    let password = &args[2];

    // tokio_core context
    let mut rt = Runtime::new().unwrap();

    // Client instance
    let (mut agent, stream) = ClientBuilder::new(jid, password)
        .set_client(ClientType::Bot, "xmpp-rs")
        .set_website("https://gitlab.com/xmpp-rs/xmpp-rs")
        .set_default_nick("bot")
        .enable_feature(ClientFeature::Avatars)
        .enable_feature(ClientFeature::ContactList)
        .enable_feature(ClientFeature::JoinRooms)
        .build()
        .unwrap();

    // We return either Some(Error) if an error was encountered
    // or None, if we were simply disconnected
    let handler = stream.map_err(Some).for_each(|evt: Event| {
        match evt {
            Event::Online => {
                println!("Online.");
            },
            Event::Disconnected => {
                println!("Disconnected.");
                return Err(None);
            },
            Event::ContactAdded(contact) => {
                println!("Contact {} added.", contact.jid);
            },
            Event::ContactRemoved(contact) => {
                println!("Contact {} removed.", contact.jid);
            },
            Event::ContactChanged(contact) => {
                println!("Contact {} changed.", contact.jid);
            },
            Event::OpenRoomBookmark(bookmark) => {
                println!("Joining room “{}” ({})…", bookmark.name, bookmark.jid);
                agent.join_room(bookmark.jid, bookmark.nick, bookmark.password, "en", "Yet another bot!");
            },
            Event::RoomJoined(jid) => {
                println!("Joined room {}.", jid);
                agent.send_message(jid.into_bare_jid(), MessageType::Groupchat, "en", "Hello world!");
            },
            Event::RoomLeft(jid) => {
                println!("Left room {}.", jid);
            },
            Event::AvatarRetrieved(jid, path) => {
                println!("Received avatar for {} in {}.", jid, path);
            },
        }
        Ok(())
    });

    rt.block_on(handler).unwrap_or_else(|e| match e {
        Some(e) => println!("Error: {:?}", e),
        None => println!("Disconnected."),
    });
}
