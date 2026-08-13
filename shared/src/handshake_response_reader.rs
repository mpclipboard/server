use crate::{
    CONNECTION_UPGRADE_HEADER, UPGRADE_MPCLIPBOARD_RAW_HEADER,
    byte_stream::ByteStream,
    http_lines_reader::{HttpLinesParser, HttpLinesReader},
    message::Message,
    strip_prefix_ignore_ascii_case,
};
use std::os::fd::AsFd;

#[must_use]
#[derive(Debug, Clone, Copy)]
struct HandshakeResponseParser {
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

    fn line_received(&mut self, line: &str) -> std::io::Result<()> {
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

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct HandshakeResponseReader {
    inner: HttpLinesReader<HandshakeResponseParser, { Message::BYTESIZE }>,
}

impl HandshakeResponseReader {
    pub fn new() -> Self {
        Self {
            inner: HttpLinesReader::new(HandshakeResponseParser::new()),
        }
    }

    pub fn read_from(
        &mut self,
        stream: &mut impl ByteStream,
        fd: &impl AsFd,
    ) -> std::io::Result<Option<((), [u8; Message::BYTESIZE], usize)>> {
        self.inner.read_from(stream, fd)
    }
}

impl Default for HandshakeResponseReader {
    fn default() -> Self {
        Self::new()
    }
}
