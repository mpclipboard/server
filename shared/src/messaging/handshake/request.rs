use crate::{
    CONNECTION_UPGRADE_HEADER, Encode, HOST_PREFIX, Host, ID, ID_PREFIX, MAX_HOST_LENGTH,
    MAX_ID_LENGTH, MAX_TOKEN_LENGTH, MIN_PADDING_LENGTH, PADDING_PREFIX, START_LINE, TOKEN_PREFIX,
    Token, UPGRADE_MPCLIPBOARD_RAW_HEADER,
    http_lines_reader::{HttpLinesParser, HttpLinesReader},
    strip_prefix_ignore_ascii_case,
};
use anyhow::{Context, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandshakeRequest {
    pub host: Host,
    pub token: Token,
    pub id: ID,
}

const _IGNORE: () = assert!(HandshakeRequest::BYTESIZE == 562);

const BASE_HANDSHAKE_LENGTH: usize = START_LINE.len() + 2 // start line
    + HOST_PREFIX.len() + 2 // Host: ...
    + TOKEN_PREFIX.len() + 2 // Token: ...
    + ID_PREFIX.len() + 2 // ID: ...
    + CONNECTION_UPGRADE_HEADER.len() + 2 // Connection: Upgrade
    + UPGRADE_MPCLIPBOARD_RAW_HEADER.len() + 2 // Upgrade: mpclipboard-raw
    + PADDING_PREFIX.len() + 2 //
    + 2; // headers end marker

impl HandshakeRequest {
    pub const BYTESIZE: usize = BASE_HANDSHAKE_LENGTH
        + MAX_HOST_LENGTH
        + MAX_TOKEN_LENGTH
        + MAX_ID_LENGTH
        + MIN_PADDING_LENGTH;
}

impl Encode<{ HandshakeRequest::BYTESIZE }> for HandshakeRequest {
    fn encode(&self, buf: &mut [u8; HandshakeRequest::BYTESIZE]) {
        let mut pos = 0;

        macro_rules! append {
            ($s:expr) => {
                buf[pos..pos + $s.len()].copy_from_slice($s.as_bytes());
                pos += $s.len();
            };
        }

        append!(START_LINE);
        append!("\r\n");

        append!(HOST_PREFIX);
        append!(self.host.as_str());
        append!("\r\n");

        append!(TOKEN_PREFIX);
        append!(self.token.as_str());
        append!("\r\n");

        append!(ID_PREFIX);
        append!(self.id.as_str());
        append!("\r\n");

        append!(CONNECTION_UPGRADE_HEADER);
        append!("\r\n");

        append!(UPGRADE_MPCLIPBOARD_RAW_HEADER);
        append!("\r\n");

        append!(PADDING_PREFIX);
        #[expect(clippy::arithmetic_side_effects)]
        let padding_len = Self::BYTESIZE - 4 - pos;
        for _ in 0..padding_len {
            append!("P");
        }
        append!("\r\n");

        append!("\r\n");

        assert_eq!(pos, Self::BYTESIZE);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HandshakeRequestParser {
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

    fn line_received(&mut self, line: &str) -> Result<()> {
        if line.starts_with(START_LINE) {
            self.seen_start_line = true;
        } else if let Some(value) = strip_prefix_ignore_ascii_case(line, HOST_PREFIX)
            && let Some(value) = value.strip_suffix("\r\n")
        {
            self.host = Some(Host::new(value).context("malformed Host header")?);
        } else if let Some(value) = strip_prefix_ignore_ascii_case(line, TOKEN_PREFIX)
            && let Some(value) = value.strip_suffix("\r\n")
        {
            self.token = Some(Token::new(value).context("malformed Token header")?);
        } else if let Some(value) = strip_prefix_ignore_ascii_case(line, ID_PREFIX)
            && let Some(value) = value.strip_suffix("\r\n")
        {
            self.id = Some(ID::new(value).context("malformed ID header")?);
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

pub type HandshakeRequestReader = HttpLinesReader<HandshakeRequestParser>;
