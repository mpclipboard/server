use crate::tls::TLS;
use anyhow::{Context as _, Result};
use mpclipboard_shared::{
    byte_stream::{ByteStream, ReadResult, WriteResult},
    event_loop::Wants,
    url::Url,
};
use rustix::io::Errno;
use rustls::{ClientConnection, pki_types::ServerName};
use std::{
    io::{ErrorKind, Read, Write},
    num::NonZeroUsize,
    os::fd::AsFd,
};

#[derive(Debug)]
pub(crate) enum Stream {
    Empty,
    Plain,
    Tls(Box<ClientConnection>),
}

#[derive(Debug)]
pub(crate) enum TlsHandshakeResult {
    Done,
    Pending,
    Died(anyhow::Error),
}

impl Stream {
    pub(crate) fn empty() -> Self {
        Self::Empty
    }

    pub(crate) fn new(url: &Url) -> Result<Self> {
        if url.is_tls() {
            let server_name = ServerName::try_from(url.host().to_owned())
                .context("failed to build TLS server name")?;
            let conn = ClientConnection::new(TLS::client_config()?, server_name)
                .context("failed to create TLS connection")?;

            Ok(Self::Tls(Box::new(conn)))
        } else {
            Ok(Self::Plain)
        }
    }

    pub(crate) fn is_tls(&self) -> bool {
        matches!(self, Self::Tls(_))
    }

    pub(crate) fn tls_handshake(&mut self, fd: &impl AsFd) -> TlsHandshakeResult {
        let conn = match self {
            Self::Tls(conn) => conn,
            Self::Plain => return TlsHandshakeResult::Done,
            Self::Empty => unreachable!("empty stream cannot perform TLS handshake"),
        };

        match conn.complete_io(&mut SocketIo(fd)) {
            Ok(_) => {
                if conn.is_handshaking() {
                    TlsHandshakeResult::Pending
                } else {
                    TlsHandshakeResult::Done
                }
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => TlsHandshakeResult::Pending,
            Err(err) => TlsHandshakeResult::Died(err.into()),
        }
    }

    pub(crate) fn flush(&mut self, fd: &impl AsFd) -> Result<()> {
        let conn = match self {
            Self::Tls(conn) => conn,
            Self::Plain => return Ok(()),
            Self::Empty => unreachable!("empty stream cannot flush"),
        };

        match conn.complete_io(&mut SocketIo(fd)) {
            Ok(_) => Ok(()),
            Err(err) if err.kind() == ErrorKind::WouldBlock => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    pub(crate) fn wants(&self, wants: Wants) -> Wants {
        match self {
            Self::Empty => unreachable!("empty stream cannot report wants"),
            Self::Plain => wants,
            Self::Tls(conn) => {
                if conn.wants_write() {
                    wants.merge(Wants::Write)
                } else {
                    wants
                }
            }
        }
    }

    pub(crate) fn tls_wants(&self) -> Wants {
        match self {
            Self::Empty => unreachable!("empty stream cannot report TLS wants"),
            Self::Plain => Wants::Write,
            Self::Tls(conn) => match (conn.wants_read(), conn.wants_write()) {
                (true, true) => Wants::ReadWrite,
                (true, false) => Wants::Read,
                (false, true) | (false, false) => Wants::Write,
            },
        }
    }
}

impl ByteStream for Stream {
    fn read_bytes(&mut self, fd: &impl AsFd, buf: &mut [u8]) -> ReadResult {
        match self {
            Self::Empty => unreachable!("empty stream cannot read"),
            Self::Plain => fd_read(fd, buf),
            Self::Tls(conn) => {
                match conn.complete_io(&mut SocketIo(fd)) {
                    Ok(_) => {}
                    Err(err) if err.kind() == ErrorKind::WouldBlock => {}
                    Err(err) => return ReadResult::Err(err.into()),
                }

                match conn.reader().read(buf).map(NonZeroUsize::new) {
                    Ok(Some(len)) => ReadResult::Data(len),
                    Ok(None) => ReadResult::Eof,
                    Err(err) if err.kind() == ErrorKind::WouldBlock => ReadResult::WouldBlock,
                    Err(err) => ReadResult::Err(err.into()),
                }
            }
        }
    }

    fn write_bytes(&mut self, fd: &impl AsFd, buf: &[u8]) -> WriteResult {
        match self {
            Self::Empty => unreachable!("empty stream cannot write"),
            Self::Plain => fd_write(fd, buf),
            Self::Tls(conn) => {
                let len = match conn.writer().write(buf).map(NonZeroUsize::new) {
                    Ok(Some(len)) => len,
                    Ok(None) => return WriteResult::Eof,
                    Err(err) if err.kind() == ErrorKind::WouldBlock => {
                        return WriteResult::WouldBlock;
                    }
                    Err(err) => return WriteResult::Err(err.into()),
                };

                match conn.complete_io(&mut SocketIo(fd)) {
                    Ok(_) => WriteResult::Data(len),
                    Err(err) if err.kind() == ErrorKind::WouldBlock => WriteResult::Data(len),
                    Err(err) => WriteResult::Err(err.into()),
                }
            }
        }
    }
}

struct SocketIo<'a, F>(&'a F);

impl<F> Read for SocketIo<'_, F>
where
    F: AsFd,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        fd_read(self.0, buf).into_io_result()
    }
}

impl<F> Write for SocketIo<'_, F>
where
    F: AsFd,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        fd_write(self.0, buf).into_io_result()
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn fd_read(fd: &impl AsFd, buf: &mut [u8]) -> ReadResult {
    match rustix::io::read(fd, buf).map(NonZeroUsize::new) {
        Ok(Some(len)) => ReadResult::Data(len),
        Ok(None) => ReadResult::Eof,
        Err(Errno::AGAIN) => ReadResult::WouldBlock,
        Err(err) => ReadResult::Err(anyhow::anyhow!("{err:?}")),
    }
}

fn fd_write(fd: &impl AsFd, buf: &[u8]) -> WriteResult {
    match rustix::io::write(fd, buf).map(NonZeroUsize::new) {
        Ok(Some(len)) => WriteResult::Data(len),
        Ok(None) => WriteResult::Eof,
        Err(Errno::AGAIN) => WriteResult::WouldBlock,
        Err(err) => WriteResult::Err(anyhow::anyhow!("{err:?}")),
    }
}

trait IntoIoResult {
    fn into_io_result(self) -> std::io::Result<usize>;
}

impl IntoIoResult for ReadResult {
    fn into_io_result(self) -> std::io::Result<usize> {
        match self {
            Self::Data(len) => Ok(len.get()),
            Self::Eof => Ok(0),
            Self::WouldBlock => Err(ErrorKind::WouldBlock.into()),
            Self::Err(err) => Err(std::io::Error::other(err)),
        }
    }
}

impl IntoIoResult for WriteResult {
    fn into_io_result(self) -> std::io::Result<usize> {
        match self {
            Self::Data(len) => Ok(len.get()),
            Self::Eof => Ok(0),
            Self::WouldBlock => Err(ErrorKind::WouldBlock.into()),
            Self::Err(err) => Err(std::io::Error::other(err)),
        }
    }
}
