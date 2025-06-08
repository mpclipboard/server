use crate::{map_of_streams::MapOfStreams, stream_id::StreamId};
use anyhow::{Context as _, Result, bail};
use futures_util::{SinkExt as _, StreamExt as _};
use mpclipboard_common::{AuthRequest, AuthResponse, Clip, Store};
use tokio::net::{TcpListener, TcpStream};
use tokio_websockets::{Message, ServerBuilder, WebSocketStream};
use uuid::Uuid;

pub(crate) struct EventLoop {
    listener: TcpListener,
    token: &'static str,

    store: Store,

    anonymous: MapOfStreams,
    authenticated: MapOfStreams,
}

impl EventLoop {
    pub(crate) fn new(listener: TcpListener, token: &'static str) -> Self {
        Self {
            listener,
            token,
            store: Store::new(),
            anonymous: MapOfStreams::new(),
            authenticated: MapOfStreams::new(),
        }
    }

    pub(crate) async fn start(&mut self) {
        loop {
            tokio::select! {
                Ok((stream, _)) = self.listener.accept() => {
                    if let Err(err) = self.on_new_connection(stream).await {
                        log::error!("{err:?}");
                    }
                }

                Some((id, message)) = self.anonymous.next() => {
                    if let Err(err) = self.on_message_from_anonymous(id, message).await {
                        log::error!("{err:?}");
                    }
                }

                Some((id, message)) = self.authenticated.next() => {
                    if let Err(err) = self.on_message_from_authenticated(id, message).await {
                        log::error!("{err:?}");
                    }
                }
            }
        }
    }

    async fn on_new_connection(&mut self, stream: TcpStream) -> Result<()> {
        let (_, ws) = ServerBuilder::new()
            .accept(stream)
            .await
            .context("failed to handle new client")?;
        let id = Uuid::new_v4();
        self.anonymous.insert(id, ws);
        Ok(())
    }

    async fn on_message_from_anonymous(&mut self, id: StreamId, message: Message) -> Result<()> {
        if message.is_ping() {
            return Ok(());
        }

        let mut ws = self.anonymous.remove(&id);

        let AuthRequest { name, token } = AuthRequest::try_from(message)?;
        if token != self.token {
            send_message(&mut ws, AuthResponse::new(false)).await;
            bail!("[auth] wrong token");
        }

        log::info!("[auth] OK {:?}", name);
        send_message(&mut ws, AuthResponse::new(true)).await;
        if let Some(clip) = self.store.current() {
            send_message(&mut ws, clip).await;
        }
        self.authenticated.insert(name, ws);

        Ok(())
    }

    async fn on_message_from_authenticated(
        &mut self,
        id: StreamId,
        message: Message,
    ) -> Result<()> {
        log::info!("[{id}] incoming message: {message:?}");

        if message.is_ping() {
            return Ok(());
        }

        if let Ok(clip) = Clip::try_from(message) {
            log::info!("[{id}] got clip {:?} at {}", clip.text, clip.timestamp);
            if self.store.add(&clip) {
                self.authenticated.broadcast(clip).await;
            } else {
                log::info!("[{id}] ignoring stale clip");
            }
        }

        Ok(())
    }
}

async fn send_message(ws: &mut WebSocketStream<TcpStream>, message: impl Into<Message>) {
    if let Err(err) = ws.send(message.into()).await {
        log::error!("[auth] failed to send message: {err:?}");
    }
}
