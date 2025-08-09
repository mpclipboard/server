use crate::{
    client::{Client, ClientMessage},
    clip::Clip,
    config::Config,
    name::Name,
    pending::{Auth, Pending},
    store::Store,
};
use anyhow::{Context as _, Result, bail};
use futures_util::StreamExt as _;
use std::time::Duration;
use tokio::{
    net::{TcpListener, TcpStream},
    time::{Interval, interval},
};
use tokio_stream::{StreamMap, StreamNotifyClose};
use tokio_websockets::ServerBuilder;
use uuid::Uuid;

pub(crate) struct EventLoop {
    listener: TcpListener,
    config: Config,

    store: Store,

    clients: StreamMap<Name, Client>,
    pending: StreamMap<Uuid, StreamNotifyClose<Pending>>,

    timer: Interval,
}

impl EventLoop {
    pub(crate) fn new(listener: TcpListener, config: Config) -> Self {
        Self {
            listener,
            config,
            store: Store::new(),
            clients: StreamMap::new(),
            pending: StreamMap::new(),
            timer: interval(Duration::from_secs(1)),
        }
    }

    pub(crate) async fn start(&mut self) {
        loop {
            tokio::select! {
                Ok((stream, _)) = self.listener.accept() => {
                    if let Err(err) = self.accept(stream).await {
                        log::error!("{err:?}");
                    }
                }

                Some((id, auth)) = self.pending.next() => {
                    if let Err(err) = self.authenticate(id, auth).await {
                        log::error!("[{id}] {err:?}")
                    }
                }

                Some((name, message)) = self.clients.next() => {
                    self.process_new_message(name, message).await;
                }

                _ = self.timer.tick() => {
                    self.ping_clients().await;
                }
            }
        }
    }

    async fn accept(&mut self, stream: TcpStream) -> Result<()> {
        let (_, ws) = ServerBuilder::new()
            .accept(stream)
            .await
            .context("failed to accept a request")?;

        let id = Uuid::new_v4();
        let conn = Pending::new(id, ws);
        log::info!("new pending client {id}");

        self.pending.insert(id, StreamNotifyClose::new(conn));
        Ok(())
    }

    async fn authenticate(&mut self, id: Uuid, auth: Option<Auth>) -> Result<()> {
        let mut conn = self
            .pending
            .remove(&id)
            .context("malformed pending state")?
            .into_inner()
            .context("stream is closed")?;

        let Some(auth) = auth else {
            return Ok(());
        };

        log::info!("auth message from client {id}: {auth:?}");

        let name = Name::new(auth.name)?;

        if auth.token == self.config.token {
            conn.reply(true).await?;
        } else {
            conn.reply(false).await?;
            bail!("invalid token in auth request");
        }

        let mut conn = conn.promote(name);

        if let Some(clip) = self.store.current() {
            conn.send_clip(&clip).await?;
        }

        self.clients.insert(name, conn);
        Ok(())
    }

    async fn process_new_message(&mut self, name: Name, message: ClientMessage) {
        match message {
            ClientMessage::Pong => {}
            ClientMessage::Clip(clip) => {
                self.process_clip(name, clip).await;
            }
        }
    }

    async fn process_clip(&mut self, name: Name, clip: Clip) {
        log::info!("[{name}] got clip {:?} at {}", clip.text, clip.timestamp);

        if self.store.add(&clip) {
            self.broadcast(clip).await;
        } else {
            log::info!("[{name}] ignoring stale clip");
        }
    }

    async fn broadcast(&mut self, clip: Clip) {
        let mut ids_to_drop = vec![];

        for (name, client) in self.clients.iter_mut() {
            if let Err(err) = client.send_clip(&clip).await {
                log::error!("[{name}] failed to broadcast clip: {err:?}");
                ids_to_drop.push(*name);
            }
        }

        for id in ids_to_drop {
            self.clients.remove(&id);
        }
    }

    async fn ping_clients(&mut self) {
        let mut ids_to_drop = vec![];

        for (name, client) in self.clients.iter_mut() {
            if let Err(err) = client.send_ping().await {
                log::error!("[{name}] {err:?}");
                ids_to_drop.push(*name);
            }
        }

        for id in ids_to_drop {
            self.clients.remove(&id);
        }
    }
}
