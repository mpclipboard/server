use crate::{
    CONNECTION_UPGRADE_HEADER, UPGRADE_MPCLIPBOARD_RAW_HEADER,
    byte_stream::ByteStream,
    http_lines_reader::{HttpLinesParser, HttpLinesReader},
    message::Message,
    strip_prefix_ignore_ascii_case,
    writer::Writer,
};
use std::os::fd::AsFd;

#[derive(Debug, Clone, Copy)]
pub struct HandshakeResponse;

impl HandshakeResponse {
    pub const BYTES: &[u8] = b"HTTP/1.1 101 Switching Protocols\r\n\
Connection: Upgrade\r\n\
Upgrade: mpclipboard-raw\r\n\
\r\n";
    pub const BYTESIZE: usize = Self::BYTES.len();
}

#[derive(Debug, Clone, Copy)]
pub struct HandshakeResponseWriter {
    inner: Writer<{ HandshakeResponse::BYTESIZE }>,
}

impl HandshakeResponseWriter {
    pub fn new() -> Self {
        let mut buf = [0; HandshakeResponse::BYTESIZE];
        buf.copy_from_slice(HandshakeResponse::BYTES);

        Self {
            inner: Writer::new(buf),
        }
    }

    pub fn write_to(
        &mut self,
        stream: &mut impl ByteStream,
        fd: &impl AsFd,
    ) -> std::io::Result<bool> {
        self.inner.write_to(stream, fd)
    }
}

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
