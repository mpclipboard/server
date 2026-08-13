use crate::{byte_stream::ByteStream, writebuf::Writebuf};
use std::os::fd::AsFd;

#[derive(Debug, Clone, Copy)]
pub struct Writer<const N: usize> {
    writebuf: Writebuf<N>,
}

impl<const N: usize> Writer<N> {
    pub(crate) const fn new(buf: [u8; N]) -> Self {
        Self {
            writebuf: Writebuf::new(buf),
        }
    }

    pub(crate) fn write_to(
        &mut self,
        stream: &mut impl ByteStream,
        fd: &impl AsFd,
    ) -> std::io::Result<bool> {
        loop {
            match stream.write_bytes(fd, self.writebuf.remainder())? {
                Some(len) => {
                    if self.writebuf.written(len) {
                        return Ok(true);
                    }
                }
                None => return Ok(false),
            }
        }
    }
}
