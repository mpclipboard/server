use crate::{Encode, writebuf::Writebuf};
use rustix::io::Errno;
use std::{marker::PhantomData, num::NonZeroUsize, os::fd::AsFd};

#[expect(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy)]
pub struct Writer<const N: usize, T>
where
    T: Encode<N>,
{
    writebuf: Writebuf<N>,
    _phantom: PhantomData<T>,
}

#[derive(Debug, PartialEq, Eq)]
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

    pub fn write(&mut self, fd: &impl AsFd) -> WriterResult {
        loop {
            let res = rustix::io::write(fd, self.writebuf.remainder()).map(NonZeroUsize::new);

            match res {
                Ok(Some(len)) => {
                    if self.writebuf.written(len) {
                        return WriterResult::Done;
                    }
                }
                Ok(None) => return WriterResult::Died(WriterError::EOF),
                Err(Errno::AGAIN) => return WriterResult::StillPending,
                Err(errno) => return WriterResult::Died(WriterError::Errno(errno)),
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum WriterError {
    EOF,
    Errno(Errno),
}
