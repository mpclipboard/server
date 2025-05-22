use crate::Config;
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_websockets::{ServerBuilder, WebSocketStream};

pub(crate) async fn start() -> Result<()> {
    let config = Box::leak(Box::new(Config::read().await?));
    log::info!("Running with config {config:?}");

    log::info!("Starting server on {}:{}", config.hostname, config.port);
    let listener = TcpListener::bind((config.hostname.clone(), config.port)).await?;

    while let Ok((stream, _)) = listener.accept().await {
        let (_, ws) = ServerBuilder::new().accept(stream).await?;

        tokio::spawn(async move {
            if let Err(err) = handle_client(ws).await {
                log::error!("{err:?}");
            }
        });
    }

    Ok(())
}

pub(crate) async fn handle_client(mut ws: WebSocketStream<TcpStream>) -> Result<()> {
    while let Some(Ok(msg)) = ws.next().await {
        if msg.is_text() {
            ws.send(msg).await?;
        }
    }

    Ok(())
}
