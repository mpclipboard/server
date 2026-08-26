use crate::connection::{
    ConnectionState, disconnected::Disconnected, maybe_tls_stream::MaybeTlsStream,
};
use mpclipboard_shared::{Message, MessageReader, MessageWriter, Wants, error};
use std::os::fd::{AsRawFd, BorrowedFd};

#[derive(Debug, Clone, Copy)]
pub struct Connected {
    fd: BorrowedFd<'static>,
    reader: MessageReader,
    writer: MessageWriter,
}

impl Connected {
    pub(crate) fn new(
        fd: BorrowedFd<'static>,
        data: &[u8],
    ) -> (Self, Option<std::io::Result<Message>>) {
        let (reader, buf) = MessageReader::new_with_data(data);

        (
            Self {
                fd,
                reader,
                writer: MessageWriter::new(),
            },
            buf,
        )
    }

    pub(crate) fn wants(&self, stream: &MaybeTlsStream) -> (BorrowedFd<'static>, Wants) {
        (
            self.fd,
            stream.wants(if self.writer.is_empty() {
                Wants::Read
            } else {
                Wants::ReadWrite
            }),
        )
    }

    pub(crate) fn push(&mut self, message: Message) {
        self.writer.push(&message);
    }

    pub(crate) fn disconnect(self, now: u64) -> ConnectionState {
        unsafe { rustix::io::close(self.fd.as_raw_fd()) };
        Disconnected::new(now).into()
    }

    pub(crate) fn read(
        mut self,
        now: u64,
        stream: &mut MaybeTlsStream,
    ) -> (ConnectionState, Option<Message>) {
        let mut buf = [0; Message::BYTESIZE];
        let needed = self.reader.bytes_needed();
        let readbuf = buf
            .get_mut(..needed)
            .unwrap_or_else(|| unreachable!("message reader requested an oversized buffer"));
        match stream.read_bytes(&self.fd, readbuf) {
            Ok(None) => (self.into(), None),
            Err(err) => {
                error!("failed to read({:?}): {err:?}", self.fd);
                (self.disconnect(now), None)
            }
            Ok(Some(len)) => {
                let data = buf
                    .get(..len.get())
                    .unwrap_or_else(|| unreachable!("stream returned an oversized read"));
                let (_, message) = self.reader.received(data);
                match message {
                    None => (self.into(), None),
                    Some(Ok(message)) => (self.into(), Some(message)),
                    Some(Err(err)) => {
                        error!("failed to decode message: {err:?}");
                        (self.disconnect(now), None)
                    }
                }
            }
        }
    }

    pub(crate) fn write(mut self, now: u64, stream: &mut MaybeTlsStream) -> ConnectionState {
        if self.writer.is_empty()
            && let Err(err) = stream.flush(&self.fd)
        {
            error!("failed to flush TLS data: {err:?}");
            return self.disconnect(now);
        }

        let Some(buf) = self.writer.remainder() else {
            return self.into();
        };
        match stream.write_bytes(&self.fd, buf) {
            Ok(Some(len)) => {
                self.writer.written(len);
                self.into()
            }
            Ok(None) => self.into(),
            Err(err) => {
                error!("failed to write({:?}): {err:?}", self.fd);
                self.disconnect(now)
            }
        }
    }
}
