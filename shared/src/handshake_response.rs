use crate::{
    CONNECTION_UPGRADE_HEADER, Encode, UPGRADE_MPCLIPBOARD_RAW_HEADER,
    http_lines_reader::HttpLinesParser, strip_prefix_ignore_ascii_case,
};
use anyhow::Result;

#[derive(Debug, Clone, Copy)]
pub struct HandshakeResponse;

impl HandshakeResponse {
    pub const BYTES: &[u8] = b"HTTP/1.1 101 Switching Protocols\r\n\
Connection: Upgrade\r\n\
Upgrade: mpclipboard-raw\r\n\
\r\n";
    pub const BYTESIZE: usize = Self::BYTES.len();
}

impl Encode<{ HandshakeResponse::BYTESIZE }> for HandshakeResponse {
    fn encode(&self, buf: &mut [u8; HandshakeResponse::BYTESIZE]) {
        buf.copy_from_slice(Self::BYTES);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HandshakeResponseParser {
    seen_start_line: bool,
    seen_connection_upgrade: bool,
    seen_upgrade_mpclipboard_raw: bool,
}

impl HttpLinesParser for HandshakeResponseParser {
    type Output = ();

    fn new() -> Self {
        Self {
            seen_start_line: false,
            seen_connection_upgrade: false,
            seen_upgrade_mpclipboard_raw: false,
        }
    }

    fn line_received(&mut self, line: &str) -> Result<()> {
        if line.starts_with("HTTP/1.1 101 Switching Protocols") {
            self.seen_start_line = true;
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
        if self.seen_start_line && self.seen_connection_upgrade && self.seen_upgrade_mpclipboard_raw
        {
            Some(())
        } else {
            None
        }
    }
}
