use anyhow::{Context as _, Result};
use futures_util::{SinkExt, Stream, StreamExt, ready};
use pin_project_lite::pin_project;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_websockets::{Message, WebSocketStream};
use uuid::Uuid;

use crate::{client::Client, name::Name};

#[derive(Deserialize, Debug)]
pub(crate) struct Auth {
    pub(crate) name: String,
    pub(crate) token: String,
}

#[derive(Serialize)]
struct AuthReply {
    success: bool,
}

pin_project! {
    pub(crate) struct Pending {
        id: Uuid,
        #[pin]
        ws: WebSocketStream<TcpStream>,
    }
}

impl Pending {
    pub(crate) fn new(id: Uuid, ws: WebSocketStream<TcpStream>) -> Self {
        Self { id, ws }
    }

    pub(crate) async fn reply(&mut self, success: bool) -> Result<()> {
        let reply = AuthReply { success };
        let json = serde_json::to_string(&reply).context("failed to serialize reply")?;
        self.ws
            .send(Message::text(json))
            .await
            .context("failed to send auth reply")
    }

    pub(crate) fn promote(self, name: Name) -> Client {
        Client::new(name, self.ws)
    }
}

impl Stream for Pending {
    type Item = Auth;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll::*;

        let this = self.project();
        let id = *this.id;
        let ws = this.ws.get_mut();

        loop {
            let message = ready!(ws.poll_next_unpin(cx));

            let Some(message) = message else {
                return Ready(None);
            };
            let message = match message {
                Ok(message) => message,
                Err(err) => {
                    log::error!("[{id}] communication error: {err:?}");
                    return Ready(None);
                }
            };

            let Some(text) = message.as_text() else {
                continue;
            };

            let auth = match serde_json::from_str::<Auth>(text) {
                Ok(auth) => auth,
                Err(err) => {
                    log::error!("[{id}] failed to parse auth message: {err:?}");
                    return Ready(None);
                }
            };

            return Ready(Some(auth));
        }
    }
}
