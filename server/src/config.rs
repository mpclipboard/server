use anyhow::{Context, Result, ensure};
use mpclipboard_shared::{Token, config::ConfigParser, url::Url};

#[derive(Clone, Copy)]
pub(crate) struct Config {
    pub(crate) url: Url,
    pub(crate) token: Token,
}

const PATH: &str = if cfg!(debug_assertions) {
    "config.toml"
} else {
    "/etc/mpclipboard-server/config.toml"
};

impl Config {
    pub(crate) fn read() -> Result<Self> {
        let [url, token] = ConfigParser::parse(PATH, ["url", "token"])?;

        let url = Url::parse(&url).context("malformed url")?;
        ensure!(!url.is_tls(), "url must have http scheme");

        let token = Token::new(&token).context("token is too long")?;

        Ok(Self { url, token })
    }
}

impl core::fmt::Debug for Config {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Config")
            .field("url", &self.url)
            .field("token", &"******")
            .finish()
    }
}
