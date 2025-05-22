use anyhow::Result;
use args::Args;
use clip::Clip;
use config::Config;

mod args;
mod clip;
mod config;
mod payload;
mod select_all_identified;
mod server;
mod ws_wrapper;

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
