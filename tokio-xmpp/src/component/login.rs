use futures::stream::StreamExt;
use tokio::io::{AsyncRead, AsyncWrite};
use xmpp_parsers::{component::Handshake, jid::Jid, ns};

use crate::{
    connect::ServerConnector,
    error::{AuthError, Error},
    proto::{Packet, XmppStream},
};

/// Log into an XMPP server as a client with a jid+pass
pub async fn component_login<C: ServerConnector>(
    connector: C,
    jid: Jid,
    password: String,
) -> Result<XmppStream<C::Stream>, Error> {
    let password = password;
    let mut xmpp_stream = connector.connect(&jid, ns::COMPONENT).await?;
    auth(&mut xmpp_stream, password).await?;
    Ok(xmpp_stream)
}

pub async fn auth<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut XmppStream<S>,
    password: String,
) -> Result<(), Error> {
    let nonza = Handshake::from_password_and_stream_id(&password, &stream.id);
    stream.send_stanza(nonza).await?;

    loop {
        match stream.next().await {
            Some(Ok(Packet::Stanza(ref stanza)))
                if stanza.is("handshake", ns::COMPONENT_ACCEPT) =>
            {
                return Ok(());
            }
            Some(Ok(Packet::Stanza(ref stanza)))
                if stanza.is("error", "http://etherx.jabber.org/streams") =>
            {
                return Err(AuthError::ComponentFail.into());
            }
            Some(_) => {}
            None => return Err(Error::Disconnected),
        }
    }
}
