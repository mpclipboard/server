use anyhow::{Context as _, Result, bail};
use common::{AuthRequest, AuthResponse, Clip, Config};
use futures_util::{SinkExt as _, StreamExt as _};
use http::Uri;
use tokio::net::TcpStream;
use tokio_websockets::{ClientBuilder, MaybeTlsStream, Message, WebSocketStream};

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    let config = Config::read()?;
    log::info!("Running with config {:?}", config);

    let name = std::env::args()
        .nth(1)
        .context("No client name was given in arguments")?;

    log::info!("Starting client {name}");

    let uri = Uri::from_static("ws://localhost:3000");
    let (mut client, _) = ClientBuilder::from_uri(uri).connect().await?;

    authenticate(&mut client, name, &config).await?;

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    let mut n = 1;

    loop {
        tokio::select! {
            Some(Ok(message)) = client.next() => {
                if let Ok(clip) = Clip::try_from(message) {
                    println!("<< {:?}", clip);
                }
            }

            _ = interval.tick() => {
                let clip = Clip::new(format!("message {n}"));
                println!(">> {:?}", clip);
                client.send(Message::from(clip)).await?;
                n += 1;
            }
            else => {
                break;
            }
        }
    }

    Ok(())
}

async fn authenticate(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    name: String,
    config: &Config,
) -> Result<()> {
    log::info!("Authenticating as {name:?}");
    let message = Message::from(AuthRequest::new(&name, &config.token));
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
