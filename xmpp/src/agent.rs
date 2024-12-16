// Copyright (c) 2023 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_xmpp::connect::ServerConnector;
pub use tokio_xmpp::parsers;
use tokio_xmpp::parsers::{disco::DiscoInfoResult, message::MessageType};
pub use tokio_xmpp::{
    jid::{BareJid, FullJid, Jid},
    minidom::Element,
    Client as TokioXmppClient,
};

use crate::{event_loop, message, muc, upload, Error, Event, RoomNick};

pub struct Agent<C: ServerConnector> {
    pub(crate) client: TokioXmppClient<C>,
    pub(crate) default_nick: Arc<RwLock<String>>,
    pub(crate) lang: Arc<Vec<String>>,
    pub(crate) disco: DiscoInfoResult,
    pub(crate) node: String,
    pub(crate) uploads: Vec<(String, Jid, PathBuf)>,
    pub(crate) awaiting_disco_bookmarks_type: bool,
    // Mapping of room->nick
    pub(crate) rooms_joined: HashMap<BareJid, String>,
    pub(crate) rooms_joining: HashMap<BareJid, String>,
    pub(crate) rooms_leaving: HashMap<BareJid, String>,
}

impl<C: ServerConnector> Agent<C> {
    pub async fn disconnect(&mut self) -> Result<(), Error> {
        self.client.send_end().await
    }

    pub async fn join_room(
        &mut self,
        room: BareJid,
        nick: Option<String>,
        password: Option<String>,
        lang: &str,
        status: &str,
    ) {
        muc::room::join_room(self, room, nick, password, lang, status).await
    }

    /// Request to leave a chatroom.
    ///
    /// If successful, an [Event::RoomLeft] event will be produced. This method does not remove the room
    /// from bookmarks nor remove the autojoin flag. See [muc::room::leave_room] for more information.
    ///
    /// # Arguments
    ///
    /// * `room_jid`: The JID of the room to leave.
    /// * `nickname`: The nickname to use in the room.
    /// * `lang`: The language of the status message (empty string when unknown).
    /// * `status`: The status message to send.
    pub async fn leave_room(
        &mut self,
        room_jid: BareJid,
        lang: impl Into<String>,
        status: impl Into<String>,
    ) {
        muc::room::leave_room(self, room_jid, lang, status).await
    }

    pub async fn send_message(
        &mut self,
        recipient: Jid,
        type_: MessageType,
        lang: &str,
        text: &str,
    ) {
        message::send::send_message(self, recipient, type_, lang, text).await
    }

    pub async fn send_room_private_message(
        &mut self,
        room: BareJid,
        recipient: RoomNick,
        lang: &str,
        text: &str,
    ) {
        muc::private_message::send_room_private_message(self, room, recipient, lang, text).await
    }

    /// Wait for new events, or Error::Disconnected when connection is closed and will not reconnect.
    pub async fn wait_for_events(&mut self) -> Vec<Event> {
        event_loop::wait_for_events(self).await
    }

    pub async fn upload_file_with(&mut self, service: &str, path: &Path) {
        upload::send::upload_file_with(self, service, path).await
    }

    /// Get the bound jid of the client.
    ///
    /// If the client is not connected, this will be None.
    pub fn bound_jid(&self) -> Option<&Jid> {
        self.client.bound_jid()
    }
}
