use futures::stream::StreamExt;
use sasl::client::mechanisms::{Anonymous, Plain, Scram};
use sasl::client::Mechanism;
use sasl::common::scram::{Sha1, Sha256};
use sasl::common::Credentials;
use std::collections::HashSet;
use std::str::FromStr;
use tokio::io::{AsyncRead, AsyncWrite};
use xmpp_parsers::sasl::{Auth, Challenge, Failure, Mechanism as XMPPMechanism, Response, Success};
use xmpp_parsers::{jid::Jid, ns};

use crate::{
    client::bind::bind,
    connect::ServerConnector,
    error::{AuthError, Error, ProtocolError},
    proto::{Packet, XmppStream},
};

pub async fn auth<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: XmppStream<S>,
    creds: Credentials,
) -> Result<S, Error> {
    let local_mechs: Vec<Box<dyn Fn() -> Box<dyn Mechanism + Send + Sync> + Send>> = vec![
        Box::new(|| Box::new(Scram::<Sha256>::from_credentials(creds.clone()).unwrap())),
        Box::new(|| Box::new(Scram::<Sha1>::from_credentials(creds.clone()).unwrap())),
        Box::new(|| Box::new(Plain::from_credentials(creds.clone()).unwrap())),
        Box::new(|| Box::new(Anonymous::new())),
    ];

    let remote_mechs: HashSet<String> = stream
        .stream_features
        .sasl_mechanisms
        .mechanisms
        .iter()
        .cloned()
        .collect();

    for local_mech in local_mechs {
        let mut mechanism = local_mech();
        if remote_mechs.contains(mechanism.name()) {
            let initial = mechanism.initial();
            let mechanism_name =
                XMPPMechanism::from_str(mechanism.name()).map_err(ProtocolError::Parsers)?;

            stream
                .send_stanza(Auth {
                    mechanism: mechanism_name,
                    data: initial,
                })
                .await?;

            loop {
                match stream.next().await {
                    Some(Ok(Packet::Stanza(stanza))) => {
                        if let Ok(challenge) = Challenge::try_from(stanza.clone()) {
                            let response = mechanism
                                .response(&challenge.data)
                                .map_err(|e| AuthError::Sasl(e))?;

                            // Send response and loop
                            stream.send_stanza(Response { data: response }).await?;
                        } else if let Ok(_) = Success::try_from(stanza.clone()) {
                            return Ok(stream.into_inner());
                        } else if let Ok(failure) = Failure::try_from(stanza.clone()) {
                            return Err(Error::Auth(AuthError::Fail(failure.defined_condition)));
                        // TODO: This code was needed for compatibility with some broken server,
                        // but it’s been forgotten which.  It is currently commented out so that we
                        // can find it and fix the server software instead.
                        /*
                        } else if stanza.name() == "failure" {
                            // Workaround for https://gitlab.com/xmpp-rs/xmpp-parsers/merge_requests/1
                            return Err(Error::Auth(AuthError::Sasl("failure".to_string())));
                        */
                        } else {
                            // ignore and loop
                        }
                    }
                    Some(Ok(_)) => {
                        // ignore and loop
                    }
                    Some(Err(e)) => return Err(e),
                    None => return Err(Error::Disconnected),
                }
            }
        }
    }

    Err(AuthError::NoMechanism.into())
}

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
