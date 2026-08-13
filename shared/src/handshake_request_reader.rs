use crate::{
    ByteStream, CONNECTION_UPGRADE_HEADER, HOST_PREFIX, HandshakeRequest, Host, ID, ID_PREFIX,
    START_LINE, TOKEN_PREFIX, Token, UPGRADE_MPCLIPBOARD_RAW_HEADER,
    http_lines_reader::{HttpLinesParser, HttpLinesReader},
    strip_prefix_ignore_ascii_case,
};
use std::os::fd::AsFd;

#[must_use]
#[derive(Debug, Clone, Copy)]
struct HandshakeRequestParser {
    seen_start_line: bool,
    host: Option<Host>,
    token: Option<Token>,
    id: Option<ID>,
    seen_connection_upgrade: bool,
    seen_upgrade_mpclipboard_raw: bool,
}

impl HttpLinesParser for HandshakeRequestParser {
    type Output = HandshakeRequest;

    fn new() -> Self {
        Self {
            seen_start_line: false,
            host: None,
            token: None,
            id: None,
            seen_connection_upgrade: false,
            seen_upgrade_mpclipboard_raw: false,
        }
    }

    fn line_received(&mut self, line: &str) -> std::io::Result<()> {
        if line.starts_with(START_LINE) {
            self.seen_start_line = true;
        } else if let Some(value) = strip_prefix_ignore_ascii_case(line, HOST_PREFIX)
            && let Some(value) = value.strip_suffix("\r\n")
        {
            self.host = Some(
                Host::new(value).ok_or_else(|| std::io::Error::other("malformed Host header"))?,
            );
        } else if let Some(value) = strip_prefix_ignore_ascii_case(line, TOKEN_PREFIX)
            && let Some(value) = value.strip_suffix("\r\n")
        {
            self.token = Some(
                Token::new(value).ok_or_else(|| std::io::Error::other("malformed Token header"))?,
            );
        } else if let Some(value) = strip_prefix_ignore_ascii_case(line, ID_PREFIX)
            && let Some(value) = value.strip_suffix("\r\n")
        {
            self.id =
                Some(ID::new(value).ok_or_else(|| std::io::Error::other("malformed Id header"))?);
        } else if strip_prefix_ignore_ascii_case(line, CONNECTION_UPGRADE_HEADER) == Some("\r\n") {
            self.seen_connection_upgrade = true;
        } else if strip_prefix_ignore_ascii_case(line, UPGRADE_MPCLIPBOARD_RAW_HEADER)
            == Some("\r\n")
        {
            self.seen_upgrade_mpclipboard_raw = true;
        }

        Ok(())
    }

    fn try_finish(&self) -> Option<Self::Output> {
        if self.seen_start_line
            && let Some(host) = self.host
            && let Some(token) = self.token
            && let Some(id) = self.id
            && self.seen_connection_upgrade
            && self.seen_upgrade_mpclipboard_raw
        {
            Some(HandshakeRequest { host, token, id })
        } else {
            None
        }
    }
}

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct HandshakeRequestReader {
    inner: HttpLinesReader<HandshakeRequestParser, { HandshakeRequest::BYTESIZE }>,
}

impl HandshakeRequestReader {
    pub fn new() -> Self {
        Self {
            inner: HttpLinesReader::new(HandshakeRequestParser::new()),
        }
    }

    pub fn read_from(
        &mut self,
        stream: &mut impl ByteStream,
        fd: &impl AsFd,
    ) -> std::io::Result<Option<(HandshakeRequest, [u8; HandshakeRequest::BYTESIZE], usize)>> {
        self.inner.read_from(stream, fd)
    }
}

impl Default for HandshakeRequestReader {
    fn default() -> Self {
        Self::new()
    }
}
