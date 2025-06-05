use anyhow::Result;
use args::Args;
use config::Config;

mod args;
mod config;
mod map_of_streams;
mod server;
mod stream_id;

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    match Args::parse()? {
        Args::Generate => {
            Config::generate();
            std::process::exit(0);
        }
        Args::Start => {
            let config: &'static Config = Box::leak(Box::new(Config::read()?));
            server::start(config).await;
        }
    }

    Ok(())
}
