use crate::{
    clip::Clip, config::Config, handshake::Handshake, multiplexer::Multiplexer, name::Name,
    store::Store,
};
use anyhow::Result;
use futures_util::{SinkExt as _, StreamExt as _};
use tokio::net::{TcpListener, TcpStream};
use tokio_websockets::Message;

pub(crate) struct EventLoop {
    listener: TcpListener,
    config: Config,

    store: Store,

    connections: Multiplexer<Name>,
}

impl EventLoop {
    pub(crate) fn new(listener: TcpListener, config: Config) -> Self {
        Self {
            listener,
            config,
            store: Store::new(),
            connections: Multiplexer::new(),
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

                Some((name, message)) = self.connections.next() => {
                    if let Err(err) = self.on_message(name, message).await {
                        log::error!("{err:?}");
                    }
                }
            }
        }
    }

    async fn on_new_connection(&mut self, stream: TcpStream) -> Result<()> {
        let mut handshake = Handshake::parse(stream).await?;
        handshake.authenticate(&self.config.token).await?;
        let (name, mut ws) = handshake.accept().await?;
        let name = Name::new(name)?;

        if let Some(clip) = self.store.current() {
            if ws.send(clip.to_message()).await.is_err() {
                log::error!("[{name}] failed to send message");
            }
        }

        self.connections.insert(name, ws);
        Ok(())
    }

    async fn on_message(&mut self, name: Name, message: Message) -> Result<()> {
        log::info!("[{name}] incoming message: {message:?}");

        if message.is_ping() || message.is_pong() {
            return Ok(());
        }

        if let Ok(clip) = Clip::from_message(&message) {
            log::info!("[{name}] got clip {:?} at {}", clip.text, clip.timestamp);
            if self.store.add(&clip) {
                self.connections.broadcast(clip).await;
            } else {
                log::info!("[{name}] ignoring stale clip");
            }
        }

        Ok(())
    }
}
