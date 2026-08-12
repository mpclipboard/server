use crate::{
    Encode,
    byte_stream::{ByteStream, WriteResult},
    writebuf::Writebuf,
};
use std::{marker::PhantomData, os::fd::AsFd};

#[expect(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy)]
pub struct Writer<const N: usize, T>
where
    T: Encode<N>,
{
    writebuf: Writebuf<N>,
    _phantom: PhantomData<T>,
}

#[derive(Debug)]
pub enum WriterResult {
    Done,
    StillPending,
    Died(WriterError),
}

impl<const N: usize, T> Writer<N, T>
where
    T: Encode<N>,
{
    pub fn new(data: &T) -> Self {
        let mut buf = [0; N];
        data.encode(&mut buf);
        Self {
            writebuf: Writebuf::new(buf),
            _phantom: PhantomData,
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
