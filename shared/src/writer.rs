use crate::{
    byte_stream::{ByteStream, WriteResult},
    writebuf::Writebuf,
};
use std::os::fd::AsFd;

#[expect(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy)]
pub struct Writer<const N: usize> {
    writebuf: Writebuf<N>,
}

#[derive(Debug)]
pub enum WriterResult {
    Done,
    StillPending,
    Died(WriterError),
}

impl<const N: usize> Writer<N> {
    pub fn new(buf: [u8; N]) -> Self {
        Self {
            writebuf: Writebuf::new(buf),
        }
    }

    pub fn write_to(&mut self, stream: &mut impl ByteStream, fd: &impl AsFd) -> WriterResult {
        loop {
            match stream.write_bytes(fd, self.writebuf.remainder()) {
                WriteResult::Data(len) => {
                    if self.writebuf.written(len) {
                        return WriterResult::Done;
                    }
                }
                WriteResult::Eof => return WriterResult::Died(WriterError::EOF),
                WriteResult::WouldBlock => return WriterResult::StillPending,
                WriteResult::Err(err) => return WriterResult::Died(WriterError::Transport(err)),
            }
        }
    }
}

#[derive(Debug)]
pub enum WriterError {
    EOF,
    Transport(anyhow::Error),
}
