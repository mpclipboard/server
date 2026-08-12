use crate::{Decode, readbuf::Readbuf, trace};
use rustix::io::Errno;
use std::{marker::PhantomData, num::NonZeroUsize, os::fd::AsFd};

#[derive(Debug, Clone, Copy)]
pub struct Reader<const N: usize, T>
where
    T: Decode<N>,
{
    readbuf: Readbuf<N>,
    _phantom: PhantomData<T>,
}

impl<const N: usize, T> Reader<N, T>
where
    T: Decode<N>,
{
    pub fn new() -> Self {
        Self {
            readbuf: Readbuf::new(),
            _phantom: PhantomData,
        }
    }

    pub fn new_with_data(data: &[u8]) -> Self {
        Self {
            readbuf: Readbuf::new_with_data(data),
            _phantom: PhantomData,
        }
    }

    pub fn read(&mut self, fd: &impl AsFd) -> ReaderResult<N, T> {
        loop {
            let res = rustix::io::read(fd, self.readbuf.remainder()).map(NonZeroUsize::new);

            match res {
                Ok(Some(len)) => {
                    trace!("received {len} bytes");
                    if let Some(buf) = self.readbuf.received(len) {
                        return match T::decode(&buf) {
                            Ok(data) => ReaderResult::Data(data),
                            Err(err) => ReaderResult::Died(ReaderError::DecodeError(err)),
                        };
                    }
                }
                Ok(None) => return ReaderResult::Died(ReaderError::EOF),
                Err(Errno::AGAIN) => return ReaderResult::StillPending,
                Err(errno) => return ReaderResult::Died(ReaderError::Errno(errno)),
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ReaderResult<const N: usize, T>
where
    T: Decode<N>,
{
    Data(T),
    StillPending,
    Died(ReaderError<N, T>),
}

#[derive(Clone, Copy)]
pub enum ReaderError<const N: usize, T>
where
    T: Decode<N>,
{
    EOF,
    DecodeError(T::Error),
    Errno(Errno),
}

impl<const N: usize, T> std::fmt::Debug for ReaderError<N, T>
where
    T: Decode<N>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EOF => write!(f, "EOF"),
            Self::DecodeError(err) => f.debug_tuple("DecodeError").field(err).finish(),
            Self::Errno(err) => f.debug_tuple("Errno").field(err).finish(),
        }
    }
}
