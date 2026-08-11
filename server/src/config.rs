use anyhow::{Context, Result, ensure};
use mpclipboard_shared::{Token, config::ConfigParser, url::Url};

#[derive(Clone, Copy)]
pub(crate) struct Config {
    pub(crate) main_url: Url,
    pub(crate) heartbeat_url: Url,
    pub(crate) token: Token,
}

const PATH: &str = if cfg!(debug_assertions) {
    "config.toml"
} else {
    "/etc/mpclipboard-server/config.toml"
};

impl Config {
    pub(crate) fn read() -> Result<Self> {
        let [main_url, heartbeat_url, token] =
            ConfigParser::parse(PATH, ["main-url", "heartbeat-url", "token"])?;

        let main_url = Url::parse(&main_url).context("malformed main-url")?;
        ensure!(!main_url.is_tls(), "main-url must have http scheme");

        let heartbeat_url = Url::parse(&heartbeat_url).context("malformed heartbeat-url")?;
        ensure!(
            !heartbeat_url.is_tls(),
            "heartbeat-url must have http scheme"
        );

        let token = Token::new(&token).context("token is too long")?;

        Ok(Self {
            main_url,
            heartbeat_url,
            token,
        })
    }
}

impl core::fmt::Debug for Config {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Config")
            .field("url", &self.main_url)
            .field("heartbeat_url", &self.heartbeat_url)
            .field("token", &"******")
            .finish()
    }
}
