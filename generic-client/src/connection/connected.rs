use crate::{
    config::Config,
    connection::{
        ConnectionState, ConnectionWants, ConnectionWantsTo, Disconnect, HasName, ReadHeartbeat,
        ReadMainConn, Tick, WriteMainConn, disconnected::Disconnected, not_supported,
    },
};
use mpclipboard_shared::{
    error,
    event_loop::Wants,
    heartbeat::Beat,
    info,
    messaging::{
        message::Message,
        writer::{MessageWriter, MessageWriterResult},
    },
    reader::{Reader, ReaderResult},
};
use std::os::fd::{AsRawFd, BorrowedFd};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Connected {
    connfd: BorrowedFd<'static>,
    heartbeatfd: BorrowedFd<'static>,
    reader: Reader<{ Message::BYTESIZE }, Message>,
    writer: MessageWriter,
    heartbeat_reader: Reader<{ Beat::BYTESIZE }, Beat>,
    last_heartbeat_at: u64,
}

impl Connected {
    pub(crate) fn new(
        now: u64,
        connfd: BorrowedFd<'static>,
        heartbeatfd: BorrowedFd<'static>,
    ) -> Self {
        Self {
            connfd,
            heartbeatfd,
            reader: Reader::new(),
            writer: MessageWriter::new(),
            heartbeat_reader: Reader::new(),
            last_heartbeat_at: now,
        }
    }

    pub(crate) fn push(&mut self, message: Message) {
        self.writer.push(&message);
    }
}

impl HasName for Connected {
    fn name(&self) -> &'static str {
        "Connected"
    }
}

impl Tick for Connected {
    fn tick(self, now: u64, _config: &Config) -> ConnectionState {
        if now - self.last_heartbeat_at > Self::FREEZE_TIME_IN_SECS {
            self.disconnect(now).into()
        } else {
            self.into()
        }
    }
}

impl Disconnect for Connected {
    fn disconnect(self, now: u64) -> ConnectionState {
        unsafe { rustix::io::close(self.connfd.as_raw_fd()) };
        unsafe { rustix::io::close(self.heartbeatfd.as_raw_fd()) };
        Disconnected::new(now).into()
    }
}

impl ConnectionWantsTo for Connected {
    fn wants(&self) -> ConnectionWants {
        ConnectionWants {
            conn: Some((
                self.connfd,
                if self.writer.is_empty() {
                    Wants::Read
                } else {
                    Wants::ReadWrite
                },
            )),
            heartbeat: Some((self.heartbeatfd, Wants::Read)),
        }
    }
}

impl ReadMainConn for Connected {
    fn read_main_conn(mut self, now: u64, _config: &Config) -> (ConnectionState, Option<Message>) {
        match self.reader.read(&self.connfd) {
            ReaderResult::StillPending => (self.into(), None),
            ReaderResult::Died(err) => {
                error!("failed to read({:?}): {err:?}", self.connfd);
                (self.disconnect(now), None)
            }
            ReaderResult::Data(message) => (self.into(), Some(message)),
        }
    }
}

impl WriteMainConn for Connected {
    fn write_main_conn(mut self, now: u64, _config: &Config) -> ConnectionState {
        match self.writer.write(&self.connfd) {
            MessageWriterResult::StillPending => self.into(),
            MessageWriterResult::Died(err) => {
                error!("failed to write({:?}): {err:?}", self.connfd);
                self.disconnect(now)
            }
        }
    }
}

impl ReadHeartbeat for Connected {
    fn read_heartbeat(mut self, now: u64) -> ConnectionState {
        match self.heartbeat_reader.read(&self.heartbeatfd) {
            ReaderResult::Data(_beat) => {
                info!("HEARTBEAT");
                self.last_heartbeat_at = now;
                self.into()
            }
            ReaderResult::StillPending => self.into(),
            ReaderResult::Died(err) => {
                error!("failed to read_heartbeat(): {err:?}");
                self.disconnect(now)
            }
        }
    }
}

not_supported!(WriteHeartbeat for Connected);
