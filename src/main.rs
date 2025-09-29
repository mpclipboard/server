#![warn(missing_docs)]
#![warn(trivial_casts, trivial_numeric_casts)]
#![warn(unused_qualifications)]
#![warn(deprecated_in_future)]
#![warn(unused_lifetimes)]
#![allow(clippy::boxed_local)]
#![doc = include_str!("../README.md")]

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

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        println!("Received interrupt, exiting...");
        std::process::exit(1);
    });

    log::info!("Starting server on http://{}:{}", config.host, config.port);
    let listener = TcpListener::bind((config.host.as_str(), config.port))
        .await
        .context("failed to bind")?;

    let mut event_loop = EventLoop::new(listener, config);
    event_loop.start().await;

    Ok(())
}
