use core::num::NonZeroUsize;

#[derive(Debug, Clone, Copy)]
pub struct HttpLinesBuffer<const N: usize> {
    buf: [u8; N],
    pos: usize,
}

impl<const N: usize> HttpLinesBuffer<N> {
    pub(crate) const fn new() -> Self {
        Self {
            buf: [0; _],
            pos: 0,
        }
    }

    pub(crate) fn remainder(&mut self) -> &mut [u8] {
        &mut self.buf[self.pos..]
    }

    pub(crate) fn received(&mut self, len: NonZeroUsize) -> std::io::Result<()> {
        let newpos = self
            .pos
            .checked_add(len.get())
            .ok_or_else(|| std::io::Error::other("HttpLinesBuffer len overflow"))?;
        if newpos > N {
            return Err(std::io::Error::other(format!(
                "HttpLinesBuffer: received() call overflows the buffer: {newpos} vs {N}"
            )));
        }
        self.pos = newpos;
        Ok(())
    }

    pub(crate) fn next_line(&self) -> Option<&[u8]> {
        let buf = &self.buf[..self.pos];
        let slash_r_slash_n_idx = buf.windows(2).position(|w| w == b"\r\n")?;
        let line_end_idx = slash_r_slash_n_idx
            .checked_add(2)
            .unwrap_or_else(|| unreachable!("bug: there must be \\r\\n after index"));
        let line = buf
            .get(..line_end_idx)
            .unwrap_or_else(|| unreachable!("bug"));
        Some(line)
    }

    pub(crate) fn consumed(&mut self, len: usize) -> std::io::Result<()> {
        let newpos = self
            .pos
            .checked_sub(len)
            .ok_or_else(|| std::io::Error::other("HttpLinesBuffer: given len > pos"))?;
        self.buf.copy_within(len..self.pos, 0);
        self.pos = newpos;
        Ok(())
    }

    pub(crate) const fn leftover(&self) -> ([u8; N], usize) {
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

        let mut buffer = HttpLinesBuffer::<128>::new();
        buffer.remainder()[..response.len()].copy_from_slice(response.as_bytes());
        buffer
            .received(NonZeroUsize::new(response.len()).unwrap())
            .unwrap();

        let res = buffer.next_line().unwrap();
        assert_eq!(res, line1.as_bytes());
        buffer.consumed(line1.len()).unwrap();

        let res = buffer.next_line().unwrap();
        assert_eq!(res, line2.as_bytes());
        buffer.consumed(line2.len()).unwrap();

        let res = buffer.next_line().unwrap();
        assert_eq!(res, line3.as_bytes());
        buffer.consumed(line3.len()).unwrap();

        let res = buffer.next_line().unwrap();
        assert_eq!(res, line4.as_bytes());
        buffer.consumed(line4.len()).unwrap();

        let res = buffer.next_line().unwrap();
        assert_eq!(res, b"\r\n");
        buffer.consumed("\r\n".len()).unwrap();

        assert_eq!(buffer.next_line(), None);

        let (buf, len) = buffer.leftover();
        assert_eq!(&buf[..len], b"ABC");
    }
}
