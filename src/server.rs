use crate::clip::Clip;
use crate::payload::Payload;
use crate::{Config, select_all_identified::SelectAllIdentified, ws_wrapper::WsWrapper};
use anyhow::{Result, bail};
use futures_util::StreamExt;
use tokio::net::TcpListener;
use tokio_websockets::ServerBuilder;
use uuid::Uuid;

pub(crate) async fn start() -> Result<()> {
    let config: &'static Config = Box::leak(Box::new(Config::read().await?));
    log::info!("Running with config {config:?}");

    log::info!("Starting server on {}:{}", config.hostname, config.port);
    let listener = TcpListener::bind((config.hostname.clone(), config.port)).await?;

    let mut anonymous = SelectAllIdentified::new();
    let mut authenticated = SelectAllIdentified::new();

    loop {
        tokio::select! {
            Ok((stream, _)) = listener.accept() => {
                let (_, ws) = ServerBuilder::new().accept(stream).await?;
                let id = Uuid::new_v4();
                let ws = WsWrapper::new(ws);
                anonymous.insert(id, ws);
            }

            Some((id, payload)) = anonymous.next() => {
                let mut ws = anonymous.remove(&id);

                if let Ok(payload) = payload {
                    match authenticate(payload, &config.token) {
                        Ok(name) => {
                            log::info!("[auth] OK {:?}", name);
                            ws.send(Payload::auth_result(true)).await;
                            authenticated.insert(name, ws);
                        },
                        Err(err) => {
                            log::info!("[auth] rejected {:?}", err);
                            ws.send(Payload::auth_result(false)).await;
                        }
                    }
                }
            }

            Some((id, payload)) = authenticated.next() => {
                if let Ok(payload) = payload {
                    match payload {
                        Payload::Clip(Clip { text, timestamp }) => {
                            log::info!("[{id}] got clip {text:?} at {timestamp}")
                        }
                        other => {
                            log::error!("[{id}] client is supposed to only send Clip messages: {other:?}");
                            authenticated.remove(&id);
                        }
                    }
                } else {
                    log::error!("[{id}] client send malformed message, disconnecting");
                    authenticated.remove(&id);
                }
            }
        }
    }
}

fn authenticate(payload: Payload, valid_token: &str) -> Result<String> {
    if let Payload::Auth { name, token } = payload {
        if token == valid_token {
            Ok(name)
        } else {
            bail!("invalid token")
        }
    } else {
        bail!("expected Authenticate message, got {payload:?}")
    }
}
