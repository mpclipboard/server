use crate::{Config, map_of_streams::MapOfStreams, store::Store};
use anyhow::{Context, Result, bail};
use common::{AuthRequest, AuthResponse, Clip};
use futures_util::{SinkExt as _, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_websockets::{Message, ServerBuilder, WebSocketStream};
use uuid::Uuid;

pub(crate) async fn start() -> Result<()> {
    let config: &'static Config = Box::leak(Box::new(Config::read()?));
    log::info!("Running with config {config:?}");

    log::info!("Starting server on {}:{}", config.hostname, config.port);
    let listener = TcpListener::bind((config.hostname.clone(), config.port)).await?;

    let mut store = Store::new();

    let mut anonymous = MapOfStreams::new();
    let mut authenticated = MapOfStreams::new();

    loop {
        tokio::select! {
            Ok((stream, _)) = listener.accept() => {
                let (_, ws) = ServerBuilder::new().accept(stream).await?;
                let id = Uuid::new_v4();
                anonymous.insert(id, ws);
            }

            Some((id, message)) = anonymous.next() => {
                let mut ws = anonymous.remove(&id);

                match authenticate(message, &config.token) {
                    Ok(name) => {
                        log::info!("[auth] OK {:?}", name);
                        auth_response(&mut ws, true).await;
                        if let Some(clip) = store.current() {
                            if let Err(err) = ws.send(Message::from(clip)).await {
                                log::error!("[auth] failed to send initial clip: {err:?}");
                            }
                        }
                        authenticated.insert(name, ws);
                    }
                    Err(err) => {
                        log::error!("[auth] ERROR {err:?}");
                        auth_response(&mut ws, false).await;
                    }
                }
            }

            Some((id, message)) = authenticated.next() => {
                match clip(message) {
                    Ok(clip) => {
                        log::info!("[{id}] got clip {:?} at {}", clip.text, clip.timestamp);
                        if store.add(clip.clone()) {
                            authenticated.broadcast(clip).await;
                        } else {
                            log::info!("[{id}] ignoring stale clip");
                        }
                    }
                    Err(err) => {
                        log::error!("[{id}] client send malformed message, disconnecting: {err:?}");
                        authenticated.remove(&id);
                    }
                }
            }
        }
    }
}

fn authenticate(message: Result<Message, tokio_websockets::Error>, token: &str) -> Result<String> {
    let message = message.context("malformed message")?;
    let auth = AuthRequest::try_from(message)?;

    if auth.token == token {
        Ok(auth.name)
    } else {
        bail!("wrong token")
    }
}

async fn auth_response(ws: &mut WebSocketStream<TcpStream>, success: bool) {
    if let Err(err) = ws.send(Message::from(AuthResponse::new(success))).await {
        log::error!("[auth] failed to send ACK back: {err:?}");
    }
}

fn clip(message: Result<Message, tokio_websockets::Error>) -> Result<Clip> {
    let message = message.context("malformed message")?;
    let clip = Clip::try_from(message)?;
    Ok(clip)
}
