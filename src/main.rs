use crate::event_loop::EventLoop;
use anyhow::{Context as _, Result};
use config::Config;
use tokio::net::TcpListener;

mod client;
mod clip;
mod config;
mod event_loop;
mod name;
mod pending;
mod store;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let config = Config::read().await?;
    log::info!("Running with config {config:?}");

    log::info!("Starting server on http://127.0.01:{}", config.port);
    let listener = TcpListener::bind(("127.0.0.1", config.port))
        .await
        .context("failed to bind")?;

    let mut event_loop = EventLoop::new(listener, config);
    event_loop.start().await;

    Ok(())
}
