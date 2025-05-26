use crate::{Config, map_of_streams::MapOfStreams, store::Store};
use anyhow::{Result, bail};
use common::{AuthRequest, AuthResponse, Clip};
use futures_util::{SinkExt as _, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_websockets::{Message, ServerBuilder, WebSocketStream};
use uuid::Uuid;

pub(crate) async fn start(config: &'static Config) {
    log::info!("Running with config {config:?}");

    log::info!("Starting server on {}:{}", config.hostname, config.port);
    let Ok(listener) = TcpListener::bind((config.hostname.clone(), config.port)).await else {
        log::error!("failed to bind to {}:{}", config.hostname, config.port);
        std::process::exit(1);
    };

    let mut store = Store::new();

    let mut anonymous = MapOfStreams::new();
    let mut authenticated = MapOfStreams::new();

    loop {
        tokio::select! {
            Ok((stream, _)) = listener.accept() => {
                match ServerBuilder::new().accept(stream).await {
                    Ok((_, ws)) => {
                        let id = Uuid::new_v4();
                        anonymous.insert(id, ws);
                    },
                    Err(err) => {
                        log::error!("failed to handle client: {err:?}");
                    }
                }
            }

            Some((id, message)) = anonymous.next() => {
                let mut ws = anonymous.remove(&id);

                match authenticate(message, &config.token) {
                    Ok(name) => {
                        log::info!("[auth] OK {:?}", name);
                        send_message(&mut ws, AuthResponse::new(true)).await;
                        if let Some(clip) = store.current() {
                            send_message(&mut ws, Message::from(clip)).await;
                        }
                        authenticated.insert(name, ws);
                    }
                    Err(err) => {
                        log::error!("[auth] ERROR {err:?}");
                        send_message(&mut ws, AuthResponse::new(false)).await;
                    }
                }
            }

            Some((id, message)) = authenticated.next() => {
                match Clip::try_from(message) {
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

fn authenticate(message: Message, token: &str) -> Result<String> {
    let auth = AuthRequest::try_from(message)?;

    if auth.token == token {
        Ok(auth.name)
    } else {
        bail!("wrong token")
    }
}

async fn send_message(ws: &mut WebSocketStream<TcpStream>, message: impl Into<Message>) {
    if let Err(err) = ws.send(message.into()).await {
        log::error!("[auth] failed to send message: {err:?}");
    }
}
