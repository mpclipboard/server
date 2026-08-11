use crate::{Host, NonEmptyInlineString};
use std::net::{SocketAddr, SocketAddrV4, ToSocketAddrs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Url {
    tls: bool,
    host: NonEmptyInlineString<256>,
    port: u16,
    header: Host,
}

impl Url {
    pub fn parse(url: &str) -> Result<Self, ParseUrlError> {
        let (scheme, url) = url
            .split_once("://")
            .ok_or(ParseUrlError::NoSeparatorBetweenSchemeAndHostPort)?;
        let (host, port) = url
            .split_once(":")
            .ok_or(ParseUrlError::NoSeparatorBetweenHostAndPort)?;

        let tls = match scheme {
            "http" => false,
            "https" => true,
            _ => return Err(ParseUrlError::UnknownScheme),
        };
        let host = NonEmptyInlineString::new(host).ok_or(ParseUrlError::HostIsTooLong)?;
        let port = port
            .parse::<u16>()
            .map_err(|_| ParseUrlError::InvalidPort)?;

        let header = NonEmptyInlineString::new(&format!("{}:{port}", host.as_str()))
            .unwrap_or_else(|| unreachable!());

        Ok(Self {
            tls,
            host,
            port,
            header,
        })
    }

    pub fn resolve(&self) -> Result<SocketAddrV4, ResolveUrlError> {
        let addrs = (self.host.as_str(), self.port)
            .to_socket_addrs()
            .map_err(ResolveUrlError::IoError)?;

        addrs
            .filter_map(|addr| match addr {
                SocketAddr::V4(v4) => Some(v4),
                SocketAddr::V6(_) => None,
            })
            .next()
            .ok_or(ResolveUrlError::CantResolveToIpV4)
    }

    pub fn is_tls(&self) -> bool {
        self.tls
    }

    pub fn header(&self) -> Host {
        self.header
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseUrlError {
    NoSeparatorBetweenSchemeAndHostPort,
    NoSeparatorBetweenHostAndPort,
    UnknownScheme,
    HostIsTooLong,
    InvalidPort,
}

impl core::fmt::Display for ParseUrlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoSeparatorBetweenSchemeAndHostPort => {
                write!(f, "no :// separator between scheme and host:port")
            }
            Self::NoSeparatorBetweenHostAndPort => {
                write!(f, "no : separator between host and port")
            }
            Self::UnknownScheme => write!(f, "unknown scheme (must be http or https)"),
            Self::HostIsTooLong => write!(f, "host is too long (max 256 bytes)"),
            Self::InvalidPort => write!(f, "invalid port"),
        }
    }
}

impl core::error::Error for ParseUrlError {}

#[derive(Debug)]
pub enum ResolveUrlError {
    IoError(std::io::Error),
    CantResolveToIpV4,
}

impl core::fmt::Display for ResolveUrlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::IoError(err) => write!(f, "IoError({err})"),
            Self::CantResolveToIpV4 => write!(f, "CantResolveToIpV4"),
        }
    }
}

impl core::error::Error for ResolveUrlError {}

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
