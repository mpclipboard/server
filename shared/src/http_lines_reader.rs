use crate::http_lines_buffer::HttpLinesBuffer;
use core::num::NonZeroUsize;

#[derive(Debug, Clone, Copy)]
pub struct HttpLinesReader<P, const N: usize>
where
    P: HttpLinesParser,
{
    buf: HttpLinesBuffer<N>,
    seen_end: bool,
    parser: P,
}

impl<P, const N: usize> HttpLinesReader<P, N>
where
    P: HttpLinesParser,
{
    pub(crate) const fn new(parser: P) -> Self {
        Self {
            buf: HttpLinesBuffer::new(),
            seen_end: false,
            parser,
        }
    }

    pub(crate) fn received(&mut self, data: &[u8]) -> std::io::Result<(usize, Option<P::Output>)> {
        let received = {
            let remainder = self.buf.remainder();
            let len = remainder.len().min(data.len());
            remainder
                .get_mut(..len)
                .unwrap_or_else(|| unreachable!("write range exceeds HTTP buffer"))
                .copy_from_slice(
                    data.get(..len)
                        .unwrap_or_else(|| unreachable!("write range exceeds input")),
                );
            if let Some(len) = NonZeroUsize::new(len) {
                self.buf.received(len)?;
            }

            len
        };

        while let Some(line) = self.buf.next_line() {
            let line = core::str::from_utf8(line).map_err(|err| {
                std::io::Error::other(format!("non-utf8 handshake line: {err:?}"))
            })?;

            if line == "\r\n" {
                self.seen_end = true;
            } else {
                self.parser.line_received(line)?;
            }
            self.buf.consumed(line.len())?;

            if self.seen_end {
                let output = self
                    .parser
                    .try_finish()
                    .ok_or_else(|| std::io::Error::other("incomplete HTTP upgrade handshake"))?;
                let consumed = received
                    .checked_sub(self.buf.len())
                    .unwrap_or_else(|| unreachable!("HTTP leftover exceeds received input"));
                return Ok((consumed, Some(output)));
            }
        }

        if received != data.len() || self.buf.remainder().is_empty() {
            return Err(std::io::Error::other("HTTP handshake exceeds buffer"));
        }

        Ok((received, None))
    }
}

pub trait HttpLinesParser {
    type Output;

    fn new() -> Self;
    fn line_received(&mut self, line: &str) -> std::io::Result<()>;
    fn try_finish(&self) -> Option<Self::Output>;
}
