use anyhow::Result;
use args::Args;
use config::Config;

mod args;
mod config;
mod server;

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    match Args::parse()? {
        Args::Generate => {
            Config::generate()?;
            std::process::exit(0);
        }
        Args::Start => {
            server::start().await?;
        }
    }

    Ok(())
}
