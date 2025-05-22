use anyhow::Result;
use args::Args;
use common::Config;

mod args;
mod map_of_streams;
mod server;
mod store;
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
            server::start().await?;
        }
    }

    Ok(())
}
