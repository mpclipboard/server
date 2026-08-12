use crate::connection::{ConnectionState, disconnected::Disconnected, stream::Stream};
use mpclipboard_shared::{
    error,
    event_loop::Wants,
    messaging::{
        message::Message,
        writer::{MessageWriter, MessageWriterResult},
    },
    reader::{Reader, ReaderResult},
};
use std::os::fd::{AsRawFd, BorrowedFd};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Connected {
    fd: BorrowedFd<'static>,
    reader: Reader<{ Message::BYTESIZE }, Message>,
    writer: MessageWriter,
}

impl Connected {
    pub(crate) fn new(
        fd: BorrowedFd<'static>,
        data: &[u8],
    ) -> (Self, Option<ReaderResult<{ Message::BYTESIZE }, Message>>) {
        let (reader, res) = Reader::new_with_data(data);

        (
            Self {
                fd,
                reader,
                writer: MessageWriter::new(),
            },
            res,
        )
    }

    pub(crate) fn wants(&self, stream: &Stream) -> Option<(BorrowedFd<'static>, Wants)> {
        Some((
            self.fd,
            stream.wants(if self.writer.is_empty() {
                Wants::Read
            } else {
                Wants::ReadWrite
            }),
        ))
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
        stream: &mut Stream,
    ) -> (ConnectionState, Option<Message>) {
        match self.reader.read_from(stream, &self.fd) {
            ReaderResult::StillPending => (self.into(), None),
            ReaderResult::Died(err) => {
                error!("failed to read({:?}): {err:?}", self.fd);
                (self.disconnect(now), None)
            }
            ReaderResult::Data(message) => (self.into(), Some(message)),
        }
    }

    pub(crate) fn write(mut self, now: u64, stream: &mut Stream) -> ConnectionState {
        if self.writer.is_empty()
            && let Err(err) = stream.flush(&self.fd)
        {
            error!("failed to flush TLS data: {err:?}");
            return self.disconnect(now);
        }

        match self.writer.write_to(stream, &self.fd) {
            MessageWriterResult::StillPending => self.into(),
            MessageWriterResult::Died(err) => {
                error!("failed to write({:?}): {err:?}", self.fd);
                self.disconnect(now)
            }
        }
    }
}
