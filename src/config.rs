use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct Config {
    pub(crate) port: u16,
    pub(crate) token: String,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("port", &self.port)
            .field("token", &"*****")
            .finish()
    }
}

const PATH: &str = if cfg!(debug_assertions) {
    "config.toml"
} else {
    "/etc/mpclipboard-server/config.toml"
};

impl Config {
    pub(crate) async fn read() -> Result<Self> {
        let content = tokio::fs::read_to_string(PATH)
            .await
            .with_context(|| format!("failed to read {PATH}"))?;
        toml::from_str(&content).context("failed to parse config")
    }
}
