use anyhow::{Context as _, Result, bail};
use futures_util::{SinkExt as _, StreamExt as _};
use http::Uri;
use shared_clipboard_server::{Config, Payload};
use tokio::net::TcpStream;
use tokio_websockets::{ClientBuilder, MaybeTlsStream, Message, WebSocketStream};

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    let config = Config::read().await?;
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
                let text = message.as_text().unwrap();
                let payload: Payload = serde_json::from_str(text).unwrap();
                println!("<< {:?}", payload);
            }

            _ = interval.tick() => {
                let payload = Payload::clip(format!("message {n}"));
                let message = Message::from(payload);
                println!(">> {:?}", message);
                client.send(message).await?;
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
    let payload = Payload::auth(&name, &config.token);
    let message = Message::from(payload);
    ws.send(message).await?;
    log::info!("Authentication message sent, waiting for reply...");

    let message = ws
        .next()
        .await
        .context("closed stream, no auth response")?
        .context("websocket error, no auth response")?;
    let text = message.as_text().context("non-text auth response")?;
    let payload: Payload = serde_json::from_str(text).context("malformed auth response")?;

    if let Payload::AuthResult { success } = payload {
        if success {
            Ok(())
        } else {
            bail!("auth failed")
        }
    } else {
        bail!(
            "expected Payload::AuthenticateAck response, got {:?}",
            payload
        )
    }
}
