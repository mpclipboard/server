use rustix::io::Errno;
use std::{io::ErrorKind, num::NonZeroUsize, os::fd::AsFd};

pub trait ByteStream {
    fn read_bytes(
        &mut self,
        fd: &impl AsFd,
        buf: &mut [u8],
    ) -> std::io::Result<Option<NonZeroUsize>>;
    fn write_bytes(&mut self, fd: &impl AsFd, buf: &[u8]) -> std::io::Result<Option<NonZeroUsize>>;
}

pub struct PlainByteStream;

impl ByteStream for PlainByteStream {
    fn read_bytes(
        &mut self,
        fd: &impl AsFd,
        buf: &mut [u8],
    ) -> std::io::Result<Option<NonZeroUsize>> {
        match rustix::io::read(fd, buf).map(NonZeroUsize::new) {
            Ok(Some(len)) => Ok(Some(len)),
            Ok(None) => Err(std::io::Error::new(ErrorKind::UnexpectedEof, "EOF")),
            Err(Errno::AGAIN) => Ok(None),
            Err(errno) => Err(errno.into()),
        }
    }

    fn write_bytes(&mut self, fd: &impl AsFd, buf: &[u8]) -> std::io::Result<Option<NonZeroUsize>> {
        match rustix::io::write(fd, buf).map(NonZeroUsize::new) {
            Ok(Some(len)) => Ok(Some(len)),
            Ok(None) => Err(std::io::Error::new(ErrorKind::UnexpectedEof, "EOF")),
            Err(Errno::AGAIN) => Ok(None),
            Err(errno) => Err(errno.into()),
        }
    }
}
