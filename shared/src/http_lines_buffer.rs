use std::num::NonZeroUsize;

use crate::message::Message;

#[derive(Debug, Clone, Copy)]
pub(crate) struct HttpLinesBuffer {
    buf: [u8; Message::BYTESIZE],
    pos: usize,
}

impl HttpLinesBuffer {
    pub(crate) fn new() -> Self {
        Self {
            buf: [0; _],
            pos: 0,
        }
    }

    pub(crate) fn remainder(&mut self) -> &mut [u8] {
        &mut self.buf[self.pos..]
    }

    pub(crate) fn received(&mut self, len: NonZeroUsize) {
        self.pos += len.get();
    }

    pub(crate) fn line(&self) -> Option<&[u8]> {
        let buf = &self.buf[..self.pos];
        let idx = buf.windows(2).position(|w| w == b"\r\n")?;
        Some(&buf[..idx + 2])
    }

    pub(crate) fn consumed(&mut self, len: usize) {
        self.buf.copy_within(len..self.pos, 0);
        self.pos -= len;
    }

    pub(crate) fn leftover(&self) -> ([u8; Message::BYTESIZE], usize) {
        (self.buf, self.pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser() {
        let line1 = "HTTP/1.1 101 Switching Protocols\r\n";
        let line2 = "Connection: Upgrade\r\n";
        let line3 = "Upgrade: mpclipboard-raw\r\n";
        let line4 = "Server: Caddy\r\n";

        let response = format!("{line1}{line2}{line3}{line4}\r\nABC");

        let mut buffer = HttpLinesBuffer::new();
        buffer.remainder()[..response.len()].copy_from_slice(response.as_bytes());
        buffer.received(NonZeroUsize::new(response.len()).unwrap());

        let res = buffer.line().unwrap();
        assert_eq!(res, line1.as_bytes());
        buffer.consumed(line1.len());

        let res = buffer.line().unwrap();
        assert_eq!(res, line2.as_bytes());
        buffer.consumed(line2.len());

        let res = buffer.line().unwrap();
        assert_eq!(res, line3.as_bytes());
        buffer.consumed(line3.len());

        let res = buffer.line().unwrap();
        assert_eq!(res, line4.as_bytes());
        buffer.consumed(line4.len());

        let res = buffer.line().unwrap();
        assert_eq!(res, b"\r\n");
        buffer.consumed("\r\n".len());

        assert_eq!(buffer.line(), None);

        let (buf, len) = buffer.leftover();
        assert_eq!(&buf[..len], b"ABC");
    }
}
