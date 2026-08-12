use rustix::io::Errno;
use std::{num::NonZeroUsize, os::fd::AsFd};

#[derive(Debug)]
pub enum ReadResult {
    Data(NonZeroUsize),
    Eof,
    WouldBlock,
    Err(anyhow::Error),
}

#[derive(Debug)]
pub enum WriteResult {
    Data(NonZeroUsize),
    Eof,
    WouldBlock,
    Err(anyhow::Error),
}

pub trait ByteStream {
    fn read_bytes(&mut self, fd: &impl AsFd, buf: &mut [u8]) -> ReadResult;
    fn write_bytes(&mut self, fd: &impl AsFd, buf: &[u8]) -> WriteResult;
}

pub struct PlainByteStream;

impl ByteStream for PlainByteStream {
    fn read_bytes(&mut self, fd: &impl AsFd, buf: &mut [u8]) -> ReadResult {
        match rustix::io::read(fd, buf).map(NonZeroUsize::new) {
            Ok(Some(len)) => ReadResult::Data(len),
            Ok(None) => ReadResult::Eof,
            Err(Errno::AGAIN) => ReadResult::WouldBlock,
            Err(err) => ReadResult::Err(anyhow::anyhow!("{err:?}")),
        }
    }

    fn write_bytes(&mut self, fd: &impl AsFd, buf: &[u8]) -> WriteResult {
        match rustix::io::write(fd, buf).map(NonZeroUsize::new) {
            Ok(Some(len)) => WriteResult::Data(len),
            Ok(None) => WriteResult::Eof,
            Err(Errno::AGAIN) => WriteResult::WouldBlock,
            Err(err) => WriteResult::Err(anyhow::anyhow!("{err:?}")),
        }
    }
}
