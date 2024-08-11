use futures::{task::Poll, Future, Sink, Stream};
use std::mem::replace;
use std::pin::Pin;
use std::task::Context;
use tokio::task::JoinHandle;

use crate::{
    client::login::client_login,
    connect::{AsyncReadAndWrite, ServerConnector},
    error::{Error, ProtocolError},
    proto::{Packet, XmppStream},
    Client, Event,
};

pub(crate) enum ClientState<S: AsyncReadAndWrite> {
    Invalid,
    Disconnected,
    Connecting(JoinHandle<Result<XmppStream<S>, Error>>),
    Connected(XmppStream<S>),
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
                Poll::Ready(Ok(Ok(stream))) => {
                    let bound_jid = stream.jid.clone();
                    self.state = ClientState::Connected(stream);
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
            ClientState::Connected(mut stream) => {
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
                        Poll::Ready(None) => {
                            // EOF
                            self.state = ClientState::Disconnected;
                            return Poll::Ready(Some(Event::Disconnected(Error::Disconnected)));
                        }
                        Poll::Ready(Some(Ok(Packet::Stanza(stanza)))) => {
                            // Receive stanza
                            self.state = ClientState::Connected(stream);
                            return Poll::Ready(Some(Event::Stanza(stanza)));
                        }
                        Poll::Ready(Some(Ok(Packet::Text(_)))) => {
                            // Ignore text between stanzas
                        }
                        Poll::Ready(Some(Ok(Packet::StreamStart(_)))) => {
                            // <stream:stream>
                            self.state = ClientState::Disconnected;
                            return Poll::Ready(Some(Event::Disconnected(
                                ProtocolError::InvalidStreamStart.into(),
                            )));
                        }
                        Poll::Ready(Some(Ok(Packet::StreamEnd))) => {
                            // End of stream: </stream:stream>
                            self.state = ClientState::Disconnected;
                            return Poll::Ready(Some(Event::Disconnected(Error::Disconnected)));
                        }
                        Poll::Pending => {
                            // Try again later
                            self.state = ClientState::Connected(stream);
                            return Poll::Pending;
                        }
                        Poll::Ready(Some(Err(e))) => {
                            self.state = ClientState::Disconnected;
                            return Poll::Ready(Some(Event::Disconnected(e.into())));
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
impl<C: ServerConnector> Sink<Packet> for Client<C> {
    type Error = Error;

    fn start_send(mut self: Pin<&mut Self>, item: Packet) -> Result<(), Self::Error> {
        match self.state {
            ClientState::Connected(ref mut stream) => {
                Pin::new(stream).start_send(item).map_err(|e| e.into())
            }
            _ => Err(Error::InvalidState),
        }
    }

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        match self.state {
            ClientState::Connected(ref mut stream) => {
                Pin::new(stream).poll_ready(cx).map_err(|e| e.into())
            }
            _ => Poll::Pending,
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        match self.state {
            ClientState::Connected(ref mut stream) => {
                Pin::new(stream).poll_flush(cx).map_err(|e| e.into())
            }
            _ => Poll::Pending,
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        match self.state {
            ClientState::Connected(ref mut stream) => {
                Pin::new(stream).poll_close(cx).map_err(|e| e.into())
            }
            _ => Poll::Pending,
        }
    }
}
