use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub hostname: String,
    pub port: u16,
    pub token: String,
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
    #[allow(dead_code)]
    pub(crate) fn generate() -> Result<()> {
        let config = Self {
            hostname: String::from("localhost"),
            port: 3000,
            token: Uuid::new_v4().to_string(),
        };
        let json = serde_json::to_string_pretty(&config)?;
        println!("{}", json);
        Ok(())
    }

    pub async fn read() -> Result<Self> {
        let path = if cfg!(debug_assertions) {
            "config.json"
        } else {
            "/etc/shared-clipboard-server/config.json"
        };

        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("failed to read {path}"))?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }
}
