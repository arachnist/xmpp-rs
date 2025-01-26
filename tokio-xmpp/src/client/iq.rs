// Copyright (c) 2025 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use alloc::collections::BTreeMap;
use alloc::sync::{Arc, Weak};
use core::error::Error;
use core::fmt;
use core::future::Future;
use core::ops::ControlFlow;
use core::pin::Pin;
use core::task::{ready, Context, Poll};
use std::io;
use std::sync::Mutex;

use futures::Stream;
use tokio::sync::oneshot;

use xmpp_parsers::{
    iq::{Iq, IqType},
    stanza_error::StanzaError,
};

use crate::{
    event::make_id,
    jid::Jid,
    minidom::Element,
    stanzastream::{StanzaState, StanzaToken},
};

/// An IQ request payload
pub enum IqRequest {
    /// Payload for a `type="get"` request
    Get(Element),

    /// Payload for a `type="set"` request
    Set(Element),
}

impl From<IqRequest> for IqType {
    fn from(other: IqRequest) -> IqType {
        match other {
            IqRequest::Get(v) => Self::Get(v),
            IqRequest::Set(v) => Self::Set(v),
        }
    }
}

/// An IQ response payload
pub enum IqResponse {
    /// Payload for a `type="result"` response.
    Result(Option<Element>),

    /// Payload for a `type="error"` response.
    Error(StanzaError),
}

impl From<IqResponse> for IqType {
    fn from(other: IqResponse) -> IqType {
        match other {
            IqResponse::Result(v) => Self::Result(v),
            IqResponse::Error(v) => Self::Error(v),
        }
    }
}

/// Error enumeration for Iq sending failures
#[derive(Debug)]
pub enum IqFailure {
    /// Internal error inside tokio_xmpp which caused the stream worker to
    /// drop the token before the response was received.
    ///
    /// Most likely, this means that the stream has died with a panic.
    LostWorker,

    /// The IQ failed to send because of an I/O or serialisation error.
    SendError(io::Error),
}

impl fmt::Display for IqFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::LostWorker => {
                f.write_str("disconnected from internal connection worker while sending IQ")
            }
            Self::SendError(e) => write!(f, "send error: {e}"),
        }
    }
}

impl Error for IqFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SendError(ref e) => Some(e),
            Self::LostWorker => None,
        }
    }
}

type IqKey = (Option<Jid>, String);
type IqMap = BTreeMap<IqKey, IqResponseSink>;

struct IqMapEntryHandle {
    key: IqKey,
    map: Weak<Mutex<IqMap>>,
}

impl Drop for IqMapEntryHandle {
    fn drop(&mut self) {
        let Some(map) = self.map.upgrade() else {
            return;
        };
        let Some(mut map) = map.lock().ok() else {
            return;
        };
        map.remove(&self.key);
    }
}

pin_project_lite::pin_project! {
    /// Handle for awaiting an IQ response.
    ///
    /// The `IqResponseToken` can be awaited and will generate a result once
    /// the Iq response has been received. Note that an `Ok(_)` result does
    /// **not** imply a successful execution of the remote command: It may
    /// contain a [`IqResponse::Error`] variant.
    ///
    /// Note that there are no internal timeouts for Iq responses: If a reply
    /// never arrives, the [`IqResponseToken`] future will never complete.
    /// Most of the time, you should combine that token with something like
    /// [`tokio::time::timeout`].
    ///
    /// Dropping (cancelling) an `IqResponseToken` removes the internal
    /// bookkeeping required for tracking the response.
    pub struct IqResponseToken {
        entry: Option<IqMapEntryHandle>,
        #[pin]
        stanza_token: Option<tokio_stream::wrappers::WatchStream<StanzaState>>,
        #[pin]
        inner: oneshot::Receiver<Result<IqResponse, IqFailure>>,
    }
}

impl IqResponseToken {
    /// Tie a stanza token to this IQ response token.
    ///
    /// The stanza token should point at the IQ **request**, the response of
    /// which this response token awaits.
    ///
    /// Awaiting the response token will then handle error states in the
    /// stanza token and return IqFailure as appropriate.
    pub(crate) fn set_stanza_token(&mut self, token: StanzaToken) {
        assert!(self.stanza_token.is_none());
        self.stanza_token = Some(token.into_stream());
    }
}

impl Future for IqResponseToken {
    type Output = Result<IqResponse, IqFailure>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        match this.inner.poll(cx) {
            Poll::Ready(Ok(v)) => {
                // Drop the map entry handle to release some memory.
                this.entry.take();
                return Poll::Ready(v);
            }
            Poll::Ready(Err(_)) => {
                log::warn!("IqResponseToken oneshot::Receiver returned receive error!");
                // Drop the map entry handle to release some memory.
                this.entry.take();
                return Poll::Ready(Err(IqFailure::LostWorker));
            }
            Poll::Pending => (),
        };

        loop {
            match this.stanza_token.as_mut().as_pin_mut() {
                // We have a stanza token to look at, so we check its state.
                Some(stream) => match ready!(stream.poll_next(cx)) {
                    // Still in the queue.
                    Some(StanzaState::Queued) => (),

                    Some(StanzaState::Dropped) | None => {
                        log::warn!("StanzaToken associated with IqResponseToken signalled that the Stanza was dropped before transmission.");
                        // Drop the map entry handle to release some memory.
                        this.entry.take();
                        // Lost stanza stream: cannot ever get a reply.
                        return Poll::Ready(Err(IqFailure::LostWorker));
                    }

                    Some(StanzaState::Failed { error }) => {
                        // Drop the map entry handle to release some memory.
                        this.entry.take();
                        // Send error: cannot ever get a reply.
                        return Poll::Ready(Err(IqFailure::SendError(error.into_io_error())));
                    }

                    Some(StanzaState::Sent { .. }) | Some(StanzaState::Acked { .. }) => {
                        // Sent successfully, stop polling the stream: We do
                        // not care what happens after successful sending,
                        // the next step we expect is that this.inner
                        // completes.
                        *this.stanza_token = None;
                        return Poll::Pending;
                    }
                },

                // No StanzaToken to poll, so we return Poll::Pending and hope
                // that we will get a response through this.inner eventually..
                None => return Poll::Pending,
            }
        }
    }
}

struct IqResponseSink {
    inner: oneshot::Sender<Result<IqResponse, IqFailure>>,
}

impl IqResponseSink {
    fn complete(self, resp: IqResponse) {
        let _: Result<_, _> = self.inner.send(Ok(resp));
    }
}

/// Utility struct to track IQ responses.
pub struct IqResponseTracker {
    map: Arc<Mutex<IqMap>>,
}

impl IqResponseTracker {
    /// Create a new empty response tracker.
    pub fn new() -> Self {
        Self {
            map: Arc::new(Mutex::new(IqMap::new())),
        }
    }

    /// Attempt to handle an IQ stanza as IQ response.
    ///
    /// Returns the IQ stanza unharmed if it is not an IQ response matching
    /// any request which is still being tracked.
    pub fn handle_iq(&self, iq: Iq) -> ControlFlow<(), Iq> {
        let payload = match iq.payload {
            IqType::Error(error) => IqResponse::Error(error),
            IqType::Result(result) => IqResponse::Result(result),
            _ => return ControlFlow::Continue(iq),
        };
        let key = (iq.from, iq.id);
        let mut map = self.map.lock().unwrap();
        match map.remove(&key) {
            None => {
                log::trace!("not handling IQ response from {:?} with id {:?}: no active tracker for this tuple", key.0, key.1);
                ControlFlow::Continue(Iq {
                    from: key.0,
                    id: key.1,
                    to: iq.to,
                    payload: payload.into(),
                })
            }
            Some(sink) => {
                sink.complete(payload);
                ControlFlow::Break(())
            }
        }
    }

    /// Allocate a new IQ response tracking handle.
    ///
    /// This modifies the IQ to assign a unique ID.
    pub fn allocate_iq_handle(
        &self,
        from: Option<Jid>,
        to: Option<Jid>,
        req: IqRequest,
    ) -> (Iq, IqResponseToken) {
        let key = (to, make_id());
        let mut map = self.map.lock().unwrap();
        let (tx, rx) = oneshot::channel();
        let sink = IqResponseSink { inner: tx };
        assert!(map.get(&key).is_none());
        let token = IqResponseToken {
            entry: Some(IqMapEntryHandle {
                key: key.clone(),
                map: Arc::downgrade(&self.map),
            }),
            stanza_token: None,
            inner: rx,
        };
        map.insert(key.clone(), sink);
        (
            Iq {
                from,
                to: key.0,
                id: key.1,
                payload: req.into(),
            },
            token,
        )
    }
}
