//! `websocket::WebSocketServerConnector` provides a `WebSocketServerConnector` for websocket connections
use alloc::borrow::Cow;

use core::{error::Error as StdError, fmt};

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use tokio::{
    io::{AsyncRead, AsyncWrite, BufStream, ReadBuf},
    net::TcpStream,
};

use http::header::HeaderValue;

/*
use crate::{
    connect::{ChannelBinding, DnsConfig, ServerConnector},
    xmlstream::{initiate_stream, PendingFeaturesRecv, StreamHeader, Timeouts},
    Client, Component, Error,
}; */

use crate::{
    connect::{ChannelBinding, ServerConnector, ServerConnectorError},
    xmlstream::{initiate_stream, PendingFeaturesRecv, StreamHeader, Timeouts},
    Error,
};

use tokio_tungstenite::{
    connect_async, tungstenite::client::IntoClientRequest, MaybeTlsStream, WebSocketStream,
};

/// Async wrapper around WebSocketStream
pub struct AsyncWebSocketStream<S>(WebSocketStream<S>);

impl<S> AsyncRead for AsyncWebSocketStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let s = self.0.get_mut();
        Pin::new(s).poll_read(cx, buf)
    }
}

impl<S> AsyncWrite for AsyncWebSocketStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let s = self.0.get_mut();
        Pin::new(s).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let s = self.0.get_mut();
        Pin::new(s).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let s = self.0.get_mut();
        Pin::new(s).poll_shutdown(cx)
    }
}

/// Connect via WebSocket to an XMPP server
#[derive(Debug, Clone)]
pub struct WebSocketServerConnector {
    host_addr: String,
}

impl From<String> for WebSocketServerConnector {
    fn from(host_addr: String) -> Self {
        Self {
            host_addr: host_addr.to_string(),
        }
    }
}

impl WebSocketServerConnector {
    async fn get_socket(&self) -> AsyncWebSocketStream<MaybeTlsStream<TcpStream>> {
        let mut ws_request = ("wss://".to_owned() + &self.host_addr + "/xmpp-websocket")
            .into_client_request()
            .unwrap();

        let ws_origin = HeaderValue::from_str(&("https://".to_owned() + &self.host_addr))
            .expect("failed to parse origin header");
        let ws_protocol = HeaderValue::from_static("xmpp");
        ws_request.headers_mut().insert("Origin", ws_origin);
        ws_request
            .headers_mut()
            .insert("Sec-WebSocket-Protocol", ws_protocol);
        let (ws_stream, _) = connect_async(ws_request).await.expect("failed to connect");
        AsyncWebSocketStream(ws_stream)
    }
}

impl ServerConnector for WebSocketServerConnector {
    type Stream = BufStream<AsyncWebSocketStream<MaybeTlsStream<TcpStream>>>;

    async fn connect(
        &self,
        jid: &xmpp_parsers::jid::Jid,
        ns: &'static str,
        timeouts: Timeouts,
    ) -> Result<(PendingFeaturesRecv<Self::Stream>, ChannelBinding), Error> {
        let stream = BufStream::new(self.get_socket().await);
        Ok((
            initiate_stream(
                stream,
                ns,
                StreamHeader {
                    to: Some(Cow::Borrowed(jid.domain().as_str())),
                    from: None,
                    id: None,
                },
                timeouts,
            )
            .await?,
            ChannelBinding::None,
        ))
    }
}

/// WebSocket specific errors
#[derive(Debug)]
pub enum WebSocketError {}

impl fmt::Display for WebSocketError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "lol, lmao: {}", self)
    }
}

impl ServerConnectorError for WebSocketError {}
impl StdError for WebSocketError {}
