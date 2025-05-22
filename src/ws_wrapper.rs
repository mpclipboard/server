use crate::payload::Payload;
use anyhow::Result;
use futures_util::{SinkExt as _, Stream};
use pin_project_lite::pin_project;
use std::task::Poll;
use tokio::net::TcpStream;
use tokio_websockets::{Message, WebSocketStream};

pin_project! {
    pub(crate) struct WsWrapper {
        #[pin]
        ws: WebSocketStream<TcpStream>,
    }
}

impl WsWrapper {
    pub(crate) fn new(ws: WebSocketStream<TcpStream>) -> Self {
        Self { ws }
    }

    pub(crate) async fn send(&mut self, payload: Payload) {
        if let Err(err) = self.ws.send(Message::from(payload)).await {
            log::error!("failed to send message: {:?}", err);
        }
    }
}

impl Stream for WsWrapper {
    type Item = Result<Payload>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match self.project().ws.poll_next(cx) {
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(message)) => match message {
                Ok(message) => Poll::Ready(Some(Payload::try_from(message))),
                Err(err) => Poll::Ready(Some(Err(err.into()))),
            },
        }
    }
}
