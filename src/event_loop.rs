use crate::{
    client::Client,
    clip::Clip,
    config::Config,
    name::Name,
    pending::{Auth, Pending},
    store::Store,
};
use anyhow::{Context as _, Result, bail};
use futures_util::StreamExt as _;
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::StreamMap;
use tokio_websockets::ServerBuilder;
use uuid::Uuid;

pub(crate) struct EventLoop {
    listener: TcpListener,
    config: Config,

    store: Store,

    clients: StreamMap<Name, Client>,
    pending: StreamMap<Uuid, Pending>,
}

impl EventLoop {
    pub(crate) fn new(listener: TcpListener, config: Config) -> Self {
        Self {
            listener,
            config,
            store: Store::new(),
            clients: StreamMap::new(),
            pending: StreamMap::new(),
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

                Some((name, clip)) = self.clients.next() => {
                    if let Err(err) = self.process_new_clip(name, clip).await {
                        log::error!("[{name}] {err:?}")
                    }
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

        self.pending.insert(id, conn);
        Ok(())
    }

    async fn authenticate(&mut self, id: Uuid, auth: Auth) -> Result<()> {
        let mut conn = self
            .pending
            .remove(&id)
            .context("malformed pending state")?;
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
            conn.send(&clip).await?;
        }

        self.clients.insert(name, conn);
        Ok(())
    }

    async fn process_new_clip(&mut self, name: Name, clip: Clip) -> Result<()> {
        log::info!("[{name}] got clip {:?} at {}", clip.text, clip.timestamp);

        if self.store.add(&clip) {
            self.broadcast(clip).await;
        } else {
            log::info!("[{name}] ignoring stale clip");
        }

        Ok(())
    }

    async fn broadcast(&mut self, clip: Clip) {
        let mut ids_to_drop = vec![];

        for (name, conn) in self.clients.iter_mut() {
            if let Err(err) = conn.send(&clip).await {
                log::error!("[{name}] failed to broadcast clip: {err:?}");
                ids_to_drop.push(*name);
            }
        }

        for id in ids_to_drop {
            self.clients.remove(&id);
        }
    }
}
