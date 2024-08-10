use futures::{task::Poll, Future, Sink, Stream};
use std::io;
use std::mem::replace;
use std::pin::Pin;
use std::task::Context;
use tokio::task::JoinHandle;
use xmpp_parsers::{
    jid::{FullJid, Jid},
    stream_features::StreamFeatures,
};

use crate::{
    client::{login::client_login, Client},
    connect::{AsyncReadAndWrite, ServerConnector},
    error::Error,
    xmlstream::{xmpp::XmppStreamElement, ReadError, XmppStream},
    Event, Stanza,
};

pub(crate) enum ClientState<S: AsyncReadAndWrite> {
    Invalid,
    Disconnected,
    Connecting(JoinHandle<Result<(Option<FullJid>, StreamFeatures, XmppStream<S>), Error>>),
    Connected {
        stream: XmppStream<S>,
        features: StreamFeatures,
        bound_jid: Jid,
    },
}

/// Incoming XMPP events
///
/// In an `async fn` you may want to use this with `use
/// futures::stream::StreamExt;`
impl<C: ServerConnector> Stream for Client<C> {
    type Item = Event;

    /// Low-level read on the XMPP stream, allowing the underlying
    /// machinery to:
    ///
    /// * connect,
    /// * starttls,
    /// * authenticate,
    /// * bind a session, and finally
    /// * receive stanzas
    ///
    /// ...for your client
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let state = replace(&mut self.state, ClientState::Invalid);

        match state {
            ClientState::Invalid => panic!("Invalid client state"),
            ClientState::Disconnected if self.reconnect => {
                // TODO: add timeout
                let connect = tokio::spawn(client_login(
                    self.connector.clone(),
                    self.jid.clone(),
                    self.password.clone(),
                ));
                self.state = ClientState::Connecting(connect);
                self.poll_next(cx)
            }
            ClientState::Disconnected => {
                self.state = ClientState::Disconnected;
                Poll::Ready(None)
            }
            ClientState::Connecting(mut connect) => match Pin::new(&mut connect).poll(cx) {
                Poll::Ready(Ok(Ok((bound_jid, features, stream)))) => {
                    let bound_jid = bound_jid.map(Jid::from).unwrap_or_else(|| self.jid.clone());
                    self.state = ClientState::Connected {
                        stream,
                        bound_jid: bound_jid.clone(),
                        features,
                    };
                    Poll::Ready(Some(Event::Online {
                        bound_jid,
                        resumed: false,
                    }))
                }
                Poll::Ready(Ok(Err(e))) => {
                    self.state = ClientState::Disconnected;
                    return Poll::Ready(Some(Event::Disconnected(e.into())));
                }
                Poll::Ready(Err(e)) => {
                    self.state = ClientState::Disconnected;
                    panic!("connect task: {}", e);
                }
                Poll::Pending => {
                    self.state = ClientState::Connecting(connect);
                    Poll::Pending
                }
            },
            ClientState::Connected {
                mut stream,
                features,
                bound_jid,
            } => {
                // Poll sink
                match Pin::new(&mut stream).poll_ready(cx) {
                    Poll::Pending => (),
                    Poll::Ready(Ok(())) => (),
                    Poll::Ready(Err(e)) => {
                        self.state = ClientState::Disconnected;
                        return Poll::Ready(Some(Event::Disconnected(e.into())));
                    }
                };

                // Poll stream
                //
                // This needs to be a loop in order to ignore packets we don’t care about, or those
                // we want to handle elsewhere.  Returning something isn’t correct in those two
                // cases because it would signal to tokio that the XmppStream is also done, while
                // there could be additional packets waiting for us.
                //
                // The proper solution is thus a loop which we exit once we have something to
                // return.
                loop {
                    match Pin::new(&mut stream).poll_next(cx) {
                        Poll::Ready(None)
                        | Poll::Ready(Some(Err(ReadError::StreamFooterReceived))) => {
                            // EOF
                            self.state = ClientState::Disconnected;
                            return Poll::Ready(Some(Event::Disconnected(Error::Disconnected)));
                        }
                        Poll::Ready(Some(Err(ReadError::HardError(e)))) => {
                            // Treat stream as dead on I/O errors
                            self.state = ClientState::Disconnected;
                            return Poll::Ready(Some(Event::Disconnected(e.into())));
                        }
                        Poll::Ready(Some(Err(ReadError::ParseError(e)))) => {
                            // Treat stream as dead on parse errors, too (for now...)
                            self.state = ClientState::Disconnected;
                            return Poll::Ready(Some(Event::Disconnected(
                                io::Error::new(io::ErrorKind::InvalidData, e).into(),
                            )));
                        }
                        Poll::Ready(Some(Err(ReadError::SoftTimeout))) => {
                            // TODO: do something smart about this.
                        }
                        Poll::Ready(Some(Ok(XmppStreamElement::Iq(stanza)))) => {
                            // Receive stanza
                            self.state = ClientState::Connected {
                                stream,
                                features,
                                bound_jid,
                            };
                            // TODO: use specific stanza types instead of going back to elements...
                            return Poll::Ready(Some(Event::Stanza(stanza.into())));
                        }
                        Poll::Ready(Some(Ok(XmppStreamElement::Message(stanza)))) => {
                            // Receive stanza
                            self.state = ClientState::Connected {
                                stream,
                                features,
                                bound_jid,
                            };
                            // TODO: use specific stanza types instead of going back to elements...
                            return Poll::Ready(Some(Event::Stanza(stanza.into())));
                        }
                        Poll::Ready(Some(Ok(XmppStreamElement::Presence(stanza)))) => {
                            // Receive stanza
                            self.state = ClientState::Connected {
                                stream,
                                features,
                                bound_jid,
                            };
                            // TODO: use specific stanza types instead of going back to elements...
                            return Poll::Ready(Some(Event::Stanza(stanza.into())));
                        }
                        Poll::Ready(Some(Ok(_))) => {
                            // We ignore these for now.
                        }
                        Poll::Pending => {
                            // Try again later
                            self.state = ClientState::Connected {
                                stream,
                                features,
                                bound_jid,
                            };
                            return Poll::Pending;
                        }
                    }
                }
            }
        }
    }
}

/// Outgoing XMPP packets
///
/// See `send_stanza()` for an `async fn`
impl<C: ServerConnector> Sink<Stanza> for Client<C> {
    type Error = Error;

    fn start_send(mut self: Pin<&mut Self>, item: Stanza) -> Result<(), Self::Error> {
        match self.state {
            ClientState::Connected { ref mut stream, .. } => Pin::new(stream)
                .start_send(&item.into())
                .map_err(|e| e.into()),
            _ => Err(Error::InvalidState),
        }
    }

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        match self.state {
            ClientState::Connected { ref mut stream, .. } => {
                Pin::new(stream).poll_ready(cx).map_err(|e| e.into())
            }
            _ => Poll::Pending,
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        match self.state {
            ClientState::Connected { ref mut stream, .. } => {
                Pin::new(stream).poll_flush(cx).map_err(|e| e.into())
            }
            _ => Poll::Pending,
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        match self.state {
            ClientState::Connected { ref mut stream, .. } => {
                Pin::new(stream).poll_close(cx).map_err(|e| e.into())
            }
            _ => Poll::Pending,
        }
    }
}
