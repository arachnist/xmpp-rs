use std::io;

use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncBufRead, AsyncWrite};
use xmpp_parsers::bind::{BindQuery, BindResponse};
use xmpp_parsers::iq::{Iq, IqType};
use xmpp_parsers::stream_features::StreamFeatures;

use crate::error::{Error, ProtocolError};
use crate::jid::{FullJid, Jid};
use crate::xmlstream::{ReadError, XmppStream, XmppStreamElement};

const BIND_REQ_ID: &str = "resource-bind";

pub async fn bind<S: AsyncBufRead + AsyncWrite + Unpin>(
    stream: &mut XmppStream<S>,
    features: &StreamFeatures,
    jid: &Jid,
) -> Result<Option<FullJid>, Error> {
    if features.can_bind() {
        let resource = jid
            .resource()
            .and_then(|resource| Some(resource.to_string()));
        let iq = Iq::from_set(BIND_REQ_ID, BindQuery::new(resource));
        stream.send(&XmppStreamElement::Iq(iq)).await?;

        loop {
            match stream.next().await {
                Some(Ok(XmppStreamElement::Iq(iq))) if iq.id == BIND_REQ_ID => match iq.payload {
                    IqType::Result(Some(payload)) => match BindResponse::try_from(payload) {
                        Ok(v) => {
                            return Ok(Some(v.into()));
                        }
                        Err(_) => return Err(ProtocolError::InvalidBindResponse.into()),
                    },
                    _ => return Err(ProtocolError::InvalidBindResponse.into()),
                },
                Some(Ok(_)) => {}
                Some(Err(ReadError::SoftTimeout)) => {}
                Some(Err(ReadError::HardError(e))) => return Err(e.into()),
                Some(Err(ReadError::ParseError(e))) => {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, e).into())
                }
                Some(Err(ReadError::StreamFooterReceived)) | None => {
                    return Err(Error::Disconnected)
                }
            }
        }
    } else {
        // No resource binding available, do nothing.
        return Ok(None);
    }
}
