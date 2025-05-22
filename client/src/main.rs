use anyhow::{Context as _, Result, bail};
use common::{AuthRequest, AuthResponse, Clip, Config};
use futures_util::{SinkExt as _, StreamExt as _};
use http::Uri;
use profiles::{Profile, select_profile};
use tokio::net::TcpStream;
use tokio_websockets::{ClientBuilder, MaybeTlsStream, Message, WebSocketStream};

mod profiles;

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    let config = Config::read()?;
    log::info!("Running with config {:?}", config);

    let Profile {
        name,
        auth,
        interval,
        correct,
    } = select_profile();

    log::info!("Starting client with profile {}", name);

    let uri = Uri::from_static("ws://localhost:3000");
    let (mut client, _) = ClientBuilder::from_uri(uri).connect().await?;

    if auth {
        authenticate(&mut client, name, &config).await?;
    }

    let mut interval = tokio::time::interval(interval);
    let mut n = 1;

    loop {
        tokio::select! {
            message = client.next() => {
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
                client.send(message).await?;
                n += 1;
            }
        }
    }

    Ok(())
}

async fn authenticate(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    name: &str,
    config: &Config,
) -> Result<()> {
    log::info!("Authenticating as {name:?}");
    let message = Message::from(AuthRequest::new(name, &config.token));
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
