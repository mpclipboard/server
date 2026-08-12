use crate::{Host, NonEmptyInlineString};
use anyhow::{Context, Result, bail};
use std::net::{SocketAddr, SocketAddrV4, ToSocketAddrs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Url {
    tls: bool,
    host: NonEmptyInlineString<256>,
    port: u16,
    header: Host,
}

impl Url {
    pub fn parse(url: &str) -> Result<Self> {
        let (scheme, url) = url
            .split_once("://")
            .context("no :// separator in the URL")?;
        let (host, port) = url
            .split_once(":")
            .context("no : separator between host and post")?;

        let tls = match scheme {
            "http" => false,
            "https" => true,
            _ => bail!("unknown URL scheme"),
        };
        let host = NonEmptyInlineString::new(host).context("host is too long")?;
        let port = port.parse::<u16>().context("invalid port")?;

        let header = NonEmptyInlineString::new(&format!("{}:{port}", host.as_str()))
            .unwrap_or_else(|| unreachable!());

        Ok(Self {
            tls,
            host,
            port,
            header,
        })
    }

    pub fn resolve(&self) -> Result<SocketAddrV4> {
        let addrs = (self.host.as_str(), self.port).to_socket_addrs()?;

        addrs
            .filter_map(|addr| match addr {
                SocketAddr::V4(v4) => Some(v4),
                SocketAddr::V6(_) => None,
            })
            .next()
            .context("can't resolve URL to IPv4 address")
    }

    pub fn is_tls(&self) -> bool {
        self.tls
    }

    pub fn host(&self) -> &str {
        self.host.as_str()
    }

    pub fn header(&self) -> Host {
        self.header
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let url = Url::parse("http://localhost:3000").unwrap();
        assert!(!url.tls);
        assert_eq!(url.host.as_str(), "localhost");
        assert_eq!(url.port, 3000);
        assert_eq!(url.header.as_str(), "localhost:3000");

        let url = Url::parse("https://google.com:443").unwrap();
        assert!(url.tls);
        assert_eq!(url.host.as_str(), "google.com");
        assert_eq!(url.port, 443);
        assert_eq!(url.header.as_str(), "google.com:443");
    }
}
