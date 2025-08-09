use crate::event_loop::EventLoop;
use anyhow::Result;
use args::Args;
use config::Config;
use tokio::net::TcpListener;

mod args;
mod clip;
mod config;
mod event_loop;
mod handshake;
mod map_of_streams;
mod store;
mod stream_id;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    match Args::parse()? {
        Args::Generate => {
            Config::generate();
            std::process::exit(0);
        }
        Args::Start => {
            let config: &'static Config = Box::leak(Box::new(Config::read()?));
            log::info!("Running with config {config:?}");

            log::info!("Starting server on {}:{}", config.hostname, config.port);
            let Ok(listener) = TcpListener::bind((config.hostname.clone(), config.port)).await
            else {
                log::error!("failed to bind to {}:{}", config.hostname, config.port);
                std::process::exit(1);
            };

            let mut event_loop = EventLoop::new(listener, &config.token);
            event_loop.start().await
        }
    }

    Ok(())
}
