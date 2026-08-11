use anyhow::{Context as _, Result};
use mpclipboard_shared::{ID, Token, config::ConfigParser, url::Url};
use std::path::{Path, PathBuf};

/// Representation of a runtime configuration
#[derive(Clone, Copy)]
pub(crate) struct Config {
    /// URL of the main communication server
    /// (e.g. `"http://127.0.0.1:3000"` or `"https://mpclipboard.me.dev"`)
    pub(crate) main_url: Url,

    /// URL of the heartbeat server
    pub(crate) heartbeat_url: Url,

    /// Token that is used for authentication
    pub(crate) token: Token,

    /// Unique ID of the client
    /// (e.g. `"macos-old-laptop"` or `"linux-dusty-minipc"`)
    pub(crate) id: ID,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("url", &self.main_url)
            .field("heartbeat_url", &self.heartbeat_url)
            .field("token", &"******")
            .field("id", &self.id)
            .finish()
    }
}

impl Config {
    pub(crate) fn new(main_url: &str, heartbeat_url: &str, token: &str, id: &str) -> Result<Self> {
        let main_url = Url::parse(main_url).context("malformed main-url")?;
        let heartbeat_url = Url::parse(heartbeat_url).context("malformed heartbeat-url")?;
        let token = Token::new(token).context("token is too long")?;
        let id = ID::new(id).context("id is too long")?;

        Ok(Self {
            main_url,
            heartbeat_url,
            token,
            id,
        })
    }

    fn read(path: impl AsRef<Path>) -> Result<Self> {
        let [main_url, heartbeat_url, token, id] =
            ConfigParser::parse(path, ["main-url", "heartbeat-url", "token", "id"])?;

        Self::new(&main_url, &heartbeat_url, &token, &id)
    }

    pub(crate) fn read_local_file() -> Result<Self> {
        Self::read("config.toml")
    }

    pub(crate) fn read_in_xdg_config_dir() -> Result<Self> {
        let xdg_config_home = std::env::var("$XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .context("no $XDG_CONFIG_HOME is set")
            .or_else(|_err| {
                let home = std::env::var("HOME").context("no $HOME")?;
                Result::<_, anyhow::Error>::Ok(PathBuf::from(home).join(".config"))
            })
            .context("neither $XDG_CONFIG_HOME nor $HOME is set")?;

        let path = xdg_config_home.join("mpclipboard").join("config.toml");
        Self::read(path)
    }
}
