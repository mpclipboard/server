use crate::{
    byte_stream::{ByteStream, ReadResult},
    http_lines_buffer::HttpLinesBuffer,
    messaging::message::Message,
};
use anyhow::{Result, anyhow};
use std::os::fd::AsFd;

#[derive(Debug, Clone, Copy)]
pub struct HttpLinesReader<P>
where
    P: HttpLinesParser,
{
    buf: HttpLinesBuffer,
    seen_end: bool,
    parser: P,
}

impl<P> HttpLinesReader<P>
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
    ) -> HttpLinesReaderResult<P> {
        loop {
            match stream.read_bytes(fd, self.buf.remainder()) {
                ReadResult::Data(len) => {
                    self.buf.received(len);

                    while let Some(line) = self.buf.line()
                        && !self.seen_end
                    {
                        let line = match core::str::from_utf8(line) {
                            Ok(line) => line,
                            Err(err) => {
                                return HttpLinesReaderResult::Err(anyhow!(
                                    "non-utf8 handshake request: {err:?}"
                                ));
                            }
                        };

                        if line == "\r\n" {
                            self.seen_end = true;
                        } else if let Err(err) = self.parser.line_received(line) {
                            return HttpLinesReaderResult::Err(err);
                        }

                        self.buf.consumed(line.len());
                    }

                    if self.seen_end
                        && let Some(output) = self.parser.try_finish()
                    {
                        let (buf, len) = self.buf.leftover();
                        return HttpLinesReaderResult::Done { buf, len, output };
                    }
                }
                ReadResult::Eof => return HttpLinesReaderResult::Err(anyhow!("EOF")),
                ReadResult::WouldBlock => return HttpLinesReaderResult::Pending,
                ReadResult::Err(err) => {
                    return HttpLinesReaderResult::Err(anyhow!("failed to read(): {err:?}"));
                }
            }
        }
    }
}

pub enum HttpLinesReaderResult<P>
where
    P: HttpLinesParser,
{
    Done {
        buf: [u8; Message::BYTESIZE],
        len: usize,
        output: P::Output,
    },
    Pending,
    Err(anyhow::Error),
}

pub trait HttpLinesParser {
    type Output;

    fn new() -> Self;
    fn line_received(&mut self, line: &str) -> Result<()>;
    fn try_finish(&self) -> Option<Self::Output>;
}
