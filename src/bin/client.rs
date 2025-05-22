use anyhow::Result;
use futures_util::{SinkExt as _, StreamExt as _};
use http::Uri;
use shared_clipboard_server::Config;
use tokio_websockets::{ClientBuilder, Message};

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    let config = Config::read().await?;
    log::info!("Running with config {:?}", config);

    log::info!("Starting client");

    let uri = Uri::from_static("ws://localhost:3000");
    let (mut client, _) = ClientBuilder::from_uri(uri).connect().await?;

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    let mut n = 1;

    loop {
        tokio::select! {
            Some(Ok(message)) = client.next() => {
                println!("<< {:?}", message.as_text().unwrap());
            }
            _ = interval.tick() => {
                let message = Message::text(format!("message {n}"));
                println!(">> {:?}", message.as_text().unwrap());
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
