//! Components in XMPP are services/gateways that are logged into an
//! XMPP server under a JID consisting of just a domain name. They are
//! allowed to use any user and resource identifiers in their stanzas.
use futures::{task::Poll, Sink, Stream};
use std::pin::Pin;
use std::task::Context;

use crate::{
    component::Component, connect::ServerConnector, xmlstream::XmppStreamElement, Error, Stanza,
};

impl<C: ServerConnector> Stream for Component<C> {
    type Item = Stanza;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        loop {
            match Pin::new(&mut self.stream).poll_next(cx) {
                Poll::Ready(Some(Ok(XmppStreamElement::Iq(stanza)))) => {
                    return Poll::Ready(Some(Stanza::Iq(stanza)))
                }
                Poll::Ready(Some(Ok(XmppStreamElement::Message(stanza)))) => {
                    return Poll::Ready(Some(Stanza::Message(stanza)))
                }
                Poll::Ready(Some(Ok(XmppStreamElement::Presence(stanza)))) => {
                    return Poll::Ready(Some(Stanza::Presence(stanza)))
                }
                Poll::Ready(Some(Ok(_))) =>
                // unexpected
                {
                    return Poll::Ready(None)
                }
                Poll::Ready(Some(Err(_))) => return Poll::Ready(None),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl<C: ServerConnector> Sink<Stanza> for Component<C> {
    type Error = Error;

    fn start_send(mut self: Pin<&mut Self>, item: Stanza) -> Result<(), Self::Error> {
        Pin::new(&mut self.stream)
            .start_send(&item.into())
            .map_err(|e| e.into())
    }

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.stream)
            .poll_ready(cx)
            .map_err(|e| e.into())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.stream)
            .poll_flush(cx)
            .map_err(|e| e.into())
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.stream)
            .poll_close(cx)
            .map_err(|e| e.into())
    }
}
