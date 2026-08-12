use crate::{byte_stream::ByteStream, http_lines_buffer::HttpLinesBuffer};
use std::os::fd::AsFd;

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
    pub fn new(parser: P) -> Self {
        Self {
            buf: HttpLinesBuffer::new(),
            seen_end: false,
            parser,
        }
    }

    pub fn read_from(
        &mut self,
        stream: &mut impl ByteStream,
        fd: &impl AsFd,
    ) -> std::io::Result<Option<(P::Output, [u8; N], usize)>> {
        loop {
            match stream.read_bytes(fd, self.buf.remainder())? {
                Some(len) => {
                    self.buf.received(len);

                    while let Some(line) = self.buf.next_line()
                        && !self.seen_end
                    {
                        let line = match core::str::from_utf8(line) {
                            Ok(line) => line,
                            Err(err) => {
                                return Err(std::io::Error::other(format!(
                                    "non-utf8 handshake line: {err:?}"
                                )));
                            }
                        };

                        if line == "\r\n" {
                            self.seen_end = true;
                        } else {
                            self.parser.line_received(line)?;
                        }
                        self.buf.consumed(line.len());

                        if self.seen_end
                            && let Some(output) = self.parser.try_finish()
                        {
                            let (buf, len) = self.buf.leftover();
                            return Ok(Some((output, buf, len)));
                        }
                    }
                }
                None => return Ok(None),
            }
        }
    }
}

pub trait HttpLinesParser {
    type Output;

    fn new() -> Self;
    fn line_received(&mut self, line: &str) -> std::io::Result<()>;
    fn try_finish(&self) -> Option<Self::Output>;
}
