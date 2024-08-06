use sasl::common::Credentials;
use xmpp_parsers::{jid::Jid, ns};

use crate::{
    client::{auth::auth, bind::bind},
    connect::ServerConnector,
    proto::XmppStream,
    Error,
};

/// Log into an XMPP server as a client with a jid+pass
/// does channel binding if supported
pub async fn client_login<C: ServerConnector>(
    server: C,
    jid: Jid,
    password: String,
) -> Result<XmppStream<C::Stream>, Error> {
    let username = jid.node().unwrap().as_str();
    let password = password;

    let xmpp_stream = server.connect(&jid, ns::JABBER_CLIENT).await?;

    let channel_binding = C::channel_binding(xmpp_stream.stream.get_ref())?;

    let creds = Credentials::default()
        .with_username(username)
        .with_password(password)
        .with_channel_binding(channel_binding);
    // Authenticated (unspecified) stream
    let stream = auth(xmpp_stream, creds).await?;
    // Authenticated XmppStream
    let xmpp_stream = XmppStream::start(stream, jid, ns::JABBER_CLIENT.to_owned()).await?;

    // XmppStream bound to user session
    let xmpp_stream = bind(xmpp_stream).await?;
    Ok(xmpp_stream)
}
