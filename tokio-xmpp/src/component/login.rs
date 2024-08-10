use std::io;

use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncBufRead, AsyncWrite};
use xmpp_parsers::{component::Handshake, jid::Jid, ns};

use crate::component::ServerConnector;
use crate::error::{AuthError, Error};
use crate::xmlstream::{ReadError, XmppStream, XmppStreamElement};

/// Log into an XMPP server as a client with a jid+pass
pub async fn component_login<C: ServerConnector>(
    connector: C,
    jid: Jid,
    password: String,
) -> Result<XmppStream<C::Stream>, Error> {
    let password = password;
    let mut stream = connector.connect(&jid, ns::COMPONENT).await?;
    let header = stream.take_header();
    let mut stream = stream.skip_features();
    let stream_id = match header.id {
        Some(ref v) => &**v,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "stream id missing on component stream",
            )
            .into())
        }
    };
    auth(&mut stream, stream_id, &password).await?;
    Ok(stream)
}

pub async fn auth<S: AsyncBufRead + AsyncWrite + Unpin>(
    stream: &mut XmppStream<S>,
    stream_id: &str,
    password: &str,
) -> Result<(), Error> {
    let nonza = Handshake::from_password_and_stream_id(password, stream_id);
    stream
        .send(&XmppStreamElement::ComponentHandshake(nonza))
        .await?;

    loop {
        match stream.next().await {
            Some(Ok(XmppStreamElement::ComponentHandshake(_))) => {
                return Ok(());
            }
            Some(Ok(_)) => {
                return Err(AuthError::ComponentFail.into());
            }
            Some(Err(ReadError::SoftTimeout)) => (),
            Some(Err(ReadError::HardError(e))) => return Err(e.into()),
            Some(Err(ReadError::ParseError(e))) => {
                return Err(io::Error::new(io::ErrorKind::InvalidData, e).into())
            }
            Some(Err(ReadError::StreamFooterReceived)) | None => return Err(Error::Disconnected),
        }
    }
}
