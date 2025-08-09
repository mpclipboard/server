use crate::{clip::Clip, name::Name};
use anyhow::{Context as _, Result};
use futures_util::{SinkExt, Stream, StreamExt, ready};
use pin_project_lite::pin_project;
use std::pin::Pin;
use tokio::net::TcpStream;
use tokio_websockets::{Message, WebSocketStream};

pin_project! {
    pub(crate) struct Client {
        name: Name,
        #[pin]
        ws: WebSocketStream<TcpStream>,
    }
}

impl Client {
    pub(crate) fn new(name: Name, ws: WebSocketStream<TcpStream>) -> Self {
        Self { name, ws }
    }

    pub(crate) async fn send(&mut self, clip: &Clip) -> Result<()> {
        let json = serde_json::to_string(clip).context("failed to serialize clip")?;
        self.ws
            .send(Message::text(json))
            .await
            .context("failed to send clip")
    }
}

impl Stream for Client {
    type Item = Clip;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll::*;

        let this = self.project();
        let name = *this.name;
        let ws = this.ws.get_mut();

        loop {
            let Some(message) = ready!(ws.poll_next_unpin(cx)) else {
                return Ready(None);
            };

            let message = match message {
                Ok(message) => message,
                Err(err) => {
                    log::error!("[{name}] communication error: {err:?}");
                    return Ready(None);
                }
            };

            let Some(text) = message.as_text() else {
                continue;
            };

            let clip = match serde_json::from_str::<Clip>(text) {
                Ok(clip) => clip,
                Err(err) => {
                    log::error!("[{name}] communication error: {err:?}");
                    return Ready(None);
                }
            };

            return Ready(Some(clip));
        }
    }
}
