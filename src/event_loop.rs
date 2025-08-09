use crate::{
    clip::Clip, config::Config, handshake::Handshake, map_of_streams::MapOfStreams, store::Store,
    stream_id::StreamId,
};
use anyhow::Result;
use futures_util::{SinkExt as _, StreamExt as _};
use tokio::net::{TcpListener, TcpStream};
use tokio_websockets::Message;

pub(crate) struct EventLoop {
    listener: TcpListener,
    config: Config,

    store: Store,

    connections: MapOfStreams,
}

impl EventLoop {
    pub(crate) fn new(listener: TcpListener, config: Config) -> Self {
        Self {
            listener,
            config,
            store: Store::new(),
            connections: MapOfStreams::new(),
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

                Some((id, message)) = self.connections.next() => {
                    if let Err(err) = self.on_message(id, message).await {
                        log::error!("{err:?}");
                    }
                }
            }
        }
    }

    async fn on_new_connection(&mut self, stream: TcpStream) -> Result<()> {
        let mut handshake = Handshake::parse(stream).await?;
        handshake.authenticate(&self.config.token).await?;
        let (id, mut ws) = handshake.accept().await?;

        if let Some(clip) = self.store.current() {
            if ws.send(clip.to_message()).await.is_err() {
                log::error!("[{id}] failed to send message");
            }
        }

        self.connections.insert(id, ws);
        Ok(())
    }

    async fn on_message(&mut self, id: StreamId, message: Message) -> Result<()> {
        log::info!("[{id}] incoming message: {message:?}");

        if message.is_ping() || message.is_pong() {
            return Ok(());
        }

        if let Ok(clip) = Clip::from_message(&message) {
            log::info!("[{id}] got clip {:?} at {}", clip.text, clip.timestamp);
            if self.store.add(&clip) {
                self.connections.broadcast(clip).await;
            } else {
                log::info!("[{id}] ignoring stale clip");
            }
        }

        Ok(())
    }
}
