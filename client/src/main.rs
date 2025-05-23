use anyhow::{Context as _, Result, bail};
use args::Args;
use common::{AuthRequest, AuthResponse, Clip};
use futures_util::{SinkExt as _, StreamExt as _};
use profiles::Profile;
use tokio::net::TcpStream;
use tokio_websockets::{MaybeTlsStream, Message, WebSocketStream};

mod args;
mod builder;
mod profiles;

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let args = Args::parse()?;
    log::info!("Running with args {:?}", args);
    let Args {
        profile:
            Profile {
                name,
                auth,
                interval,
                correct,
            },
        url,
        token,
    } = args;

    log::info!("Starting client with profile {}", name);

    let (mut ws, response) = builder::new(&url).await?;
    log::info!("WS(S) connect response: {response:?}");

    if auth {
        authenticate(&mut ws, name, &token).await?;
    }

    let mut interval = tokio::time::interval(interval);
    let mut n = 1;

    loop {
        tokio::select! {
            message = ws.next() => {
                match message {
                    None => {
                        log::info!("connection closed, exiting");
                        break;
                    }
                    Some(Err(err)) => {
                        log::error!("got error {err:?}");
                        break;
                    }
                    Some(Ok(message)) => {
                        match Clip::try_from(message) {
                            Ok(clip) => log::info!("<< {:?}", clip),
                            Err(err) => {
                                log::error!("communication error: expected clip, got error {err:?}");
                                break;
                            }
                        }
                    }
                }
            }

            _ = interval.tick() => {
                let message = if correct {
                    let clip = Clip::new(format!("clip of {name} - {n}"));
                    log::info!(">> {:?}", clip);
                    Message::from(clip)
                } else {
                    log::info!(">> malformed data");
                    Message::text("malformed data")
                };
                ws.send(message).await?;
                n += 1;
            }
        }
    }

    Ok(())
}

async fn authenticate(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    name: &str,
    token: &str,
) -> Result<()> {
    log::info!("Authenticating as {name:?}");
    let message = Message::from(AuthRequest::new(name, token));
    ws.send(message).await?;
    log::info!("Authentication message sent, waiting for reply...");

    let message = ws
        .next()
        .await
        .context("closed stream, no auth response")?
        .context("websocket error, no auth response")?;
    let auth = AuthResponse::try_from(message)?;

    if auth.success {
        Ok(())
    } else {
        bail!("auth failed")
    }
}
