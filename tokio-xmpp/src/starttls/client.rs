use std::str::FromStr;

use xmpp_parsers::jid::Jid;

use crate::{Error, SimpleClient};

use super::ServerConfig;

impl SimpleClient<ServerConfig> {
    /// Start a new XMPP client and wait for a usable session
    pub async fn new<P: Into<String>>(jid: &str, password: P) -> Result<Self, Error> {
        let jid = Jid::from_str(jid)?;
        Self::new_with_jid(jid, password.into()).await
    }

    /// Start a new client given that the JID is already parsed.
    pub async fn new_with_jid(jid: Jid, password: String) -> Result<Self, Error> {
        Self::new_with_jid_connector(ServerConfig::UseSrv, jid, password).await
    }
}
