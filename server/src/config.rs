use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub(crate) struct Config {
    pub(crate) hostname: String,
    pub(crate) port: u16,
    pub(crate) token: String,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("hostname", &self.hostname)
            .field("port", &self.port)
            .field("token", &"*****")
            .finish()
    }
}

impl Config {
    pub(crate) fn generate() {
        let config = Self {
            hostname: String::from("localhost"),
            port: 3000,
            token: Uuid::new_v4().to_string(),
        };
        let toml = toml::to_string_pretty(&config).expect("static values, can't fail");
        println!("{}", toml);
    }

    pub(crate) fn read() -> Result<Self> {
        let path = if cfg!(debug_assertions) {
            "config.toml"
        } else {
            "/etc/shared-clipboard-server/config.toml"
        };

        let content =
            std::fs::read_to_string(path).with_context(|| format!("failed to read {path}"))?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}
