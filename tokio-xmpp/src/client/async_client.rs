use futures::{sink::SinkExt, task::Poll, Future, Sink, Stream};
use minidom::Element;
use std::mem::replace;
use std::pin::Pin;
use std::task::Context;
use tokio::task::JoinHandle;
use xmpp_parsers::{jid::Jid, ns, stream_features::StreamFeatures};

use crate::{
    client::connect::client_login,
    connect::{AsyncReadAndWrite, ServerConnector},
    error::{Error, ProtocolError},
    proto::{add_stanza_id, Packet, XmppStream},
    Event,
};

#[cfg(any(feature = "starttls", feature = "insecure-tcp"))]
use crate::connect::DnsConfig;
#[cfg(feature = "starttls")]
use crate::connect::StartTlsServerConnector;
#[cfg(feature = "insecure-tcp")]
use crate::connect::TcpServerConnector;

/// XMPP client connection and state
///
/// It is able to reconnect. TODO: implement session management.
///
/// This implements the `futures` crate's [`Stream`](#impl-Stream) and
/// [`Sink`](#impl-Sink<Packet>) traits.
pub struct Client<C: ServerConnector> {
    jid: Jid,
    password: String,
    connector: C,
    state: ClientState<C::Stream>,
    reconnect: bool,
    // TODO: tls_required=true
}

enum ClientState<S: AsyncReadAndWrite> {
    Invalid,
    Disconnected,
    Connecting(JoinHandle<Result<XmppStream<S>, Error>>),
    Connected(XmppStream<S>),
}

#[cfg(feature = "starttls")]
impl Client<StartTlsServerConnector> {
    /// Start a new XMPP client using StartTLS transport and autoreconnect
    ///
    /// Start polling the returned instance so that it will connect
    /// and yield events.
    pub fn new<J: Into<Jid>, P: Into<String>>(jid: J, password: P) -> Self {
        let jid = jid.into();
        let mut client = Self::new_starttls(
            jid.clone(),
            password,
            DnsConfig::srv(&jid.domain().to_string(), "_xmpp-client._tcp", 5222),
        );
        client.set_reconnect(true);
        client
    }

    /// Start a new XMPP client with StartTLS transport and specific DNS config
    pub fn new_starttls<J: Into<Jid>, P: Into<String>>(
        jid: J,
        password: P,
        dns_config: DnsConfig,
    ) -> Self {
        Self::new_with_connector(jid, password, StartTlsServerConnector::from(dns_config))
    }
}

#[cfg(feature = "insecure-tcp")]
impl Client<TcpServerConnector> {
    /// Start a new XMPP client with plaintext insecure connection and specific DNS config
    pub fn new_plaintext<J: Into<Jid>, P: Into<String>>(
        jid: J,
        password: P,
        dns_config: DnsConfig,
    ) -> Self {
        Self::new_with_connector(jid, password, TcpServerConnector::from(dns_config))
    }
}

impl<C: ServerConnector> Client<C> {
    /// Start a new client given that the JID is already parsed.
    pub fn new_with_connector<J: Into<Jid>, P: Into<String>>(
        jid: J,
        password: P,
        connector: C,
    ) -> Self {
        let jid = jid.into();
        let password = password.into();

        let connect = tokio::spawn(client_login(
            connector.clone(),
            jid.clone(),
            password.clone(),
        ));
        let client = Client {
            jid,
            password,
            connector,
            state: ClientState::Connecting(connect),
            reconnect: false,
        };
        client
    }

    /// Set whether to reconnect (`true`) or let the stream end
    /// (`false`) when a connection to the server has ended.
    pub fn set_reconnect(&mut self, reconnect: bool) -> &mut Self {
        self.reconnect = reconnect;
        self
    }

    /// Get the client's bound JID (the one reported by the XMPP
    /// server).
    pub fn bound_jid(&self) -> Option<&Jid> {
        match self.state {
            ClientState::Connected(ref stream) => Some(&stream.jid),
            _ => None,
        }
    }

    /// Send stanza
    pub async fn send_stanza(&mut self, stanza: Element) -> Result<(), Error> {
        self.send(Packet::Stanza(add_stanza_id(stanza, ns::JABBER_CLIENT)))
            .await
    }

    /// Get the stream features (`<stream:features/>`) of the underlying stream
    pub fn get_stream_features(&self) -> Option<&StreamFeatures> {
        match self.state {
            ClientState::Connected(ref stream) => Some(&stream.stream_features),
            _ => None,
        }
    }

    /// End connection by sending `</stream:stream>`
    ///
    /// You may expect the server to respond with the same. This
    /// client will then drop its connection.
    ///
    /// Make sure to disable reconnect.
    pub async fn send_end(&mut self) -> Result<(), Error> {
        self.send(Packet::StreamEnd).await
    }
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
