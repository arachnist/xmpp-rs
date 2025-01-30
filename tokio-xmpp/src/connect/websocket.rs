//! `websocket::WebSocketServerConnector` provides a `WebSocketServerConnector` for websocket connections

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use tokio::{
    io::{AsyncRead, AsyncWrite, BufStream, ReadBuf},
    net::TcpStream,
};

/*
use crate::{
    connect::{ChannelBinding, DnsConfig, ServerConnector},
    xmlstream::{initiate_stream, PendingFeaturesRecv, StreamHeader, Timeouts},
    Client, Component, Error,
}; */

use crate::{
    connect::{ChannelBinding, DnsConfig, ServerConnector},
    xmlstream::{
        PendingFeaturesRecv,
        Timeouts
    },
    Error,
};

use tokio_tungstenite::{
    MaybeTlsStream,
    WebSocketStream,
};

/// Connect via WebSocket to an XMPP server
#[derive(Debug, Clone)]
pub struct WebSocketServerConnector(pub DnsConfig);
impl From<DnsConfig> for WebSocketServerConnector {
    fn from(dns_config: DnsConfig) ->WebSocketServerConnector {
        Self(dns_config)
    }
}

pub struct AsyncWebSocketStream<S>(WebSocketStream<S>);

impl<S> AsyncRead for AsyncWebSocketStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin {
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
    S: AsyncRead + AsyncWrite + Unpin {
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

impl ServerConnector for WebSocketServerConnector {
    type Stream = BufStream<AsyncWebSocketStream<MaybeTlsStream<TcpStream>>>;

    async fn connect(
        &self,
        jid: &xmpp_parsers::jid::Jid,
        ns: &'static str,
        timeouts: Timeouts,
    ) -> Result<(PendingFeaturesRecv<Self::Stream>, ChannelBinding), Error> {
        let stream = BufStream::new(self.0.resolve().await?);
        Ok((
            None,
            ChannelBinding::None,
        ))
    }
}
