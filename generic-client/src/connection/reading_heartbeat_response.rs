use crate::{
    config::Config,
    connection::{
        ConnectionState, ConnectionWants, ConnectionWantsTo, Disconnect, HasName, ReadHeartbeat,
        Tick, connected::Connected, disconnected::Disconnected, not_supported,
    },
};
use mpclipboard_shared::{
    error,
    event_loop::Wants,
    heartbeat::{HeartbeatResponse, HeartbeatResponseDecodeError},
    reader::{Reader, ReaderError, ReaderResult},
    trace,
};
use std::os::fd::{AsRawFd, BorrowedFd};

#[derive(Debug, Clone, Copy)]
pub struct ReadingHeartbeatResponse {
    connfd: BorrowedFd<'static>,
    heartbeatfd: BorrowedFd<'static>,
    reader: Reader<{ HeartbeatResponse::BYTESIZE }, HeartbeatResponse>,
    last_activity_at: u64,
}

impl ReadingHeartbeatResponse {
    pub(crate) fn new(
        connfd: BorrowedFd<'static>,
        heartbeatfd: BorrowedFd<'static>,
        now: u64,
    ) -> Self {
        Self {
            connfd,
            heartbeatfd,
            reader: Reader::new(),
            last_activity_at: now,
        }
    }
}

impl HasName for ReadingHeartbeatResponse {
    fn name(&self) -> &'static str {
        "ReadingHeartbeatResponse"
    }
}

impl Tick for ReadingHeartbeatResponse {
    fn tick(self, now: u64, _config: &Config) -> ConnectionState {
        if now - self.last_activity_at > Self::FREEZE_TIME_IN_SECS {
            self.disconnect(now).into()
        } else {
            self.into()
        }
    }
}

impl Disconnect for ReadingHeartbeatResponse {
    fn disconnect(self, now: u64) -> ConnectionState {
        unsafe { rustix::io::close(self.connfd.as_raw_fd()) };
        unsafe { rustix::io::close(self.heartbeatfd.as_raw_fd()) };
        Disconnected::new(now).into()
    }
}

impl ConnectionWantsTo for ReadingHeartbeatResponse {
    fn wants(&self) -> ConnectionWants {
        ConnectionWants {
            conn: None,
            heartbeat: Some((self.heartbeatfd, Wants::Read)),
        }
    }
}

impl ReadHeartbeat for ReadingHeartbeatResponse {
    fn read_heartbeat(mut self, now: u64) -> ConnectionState {
        match self.reader.read(&self.heartbeatfd) {
            ReaderResult::Data(_res) => {
                trace!("finished heartbeat request");
                Connected::new(now, self.connfd, self.heartbeatfd).into()
            }
            ReaderResult::StillPending => {
                self.last_activity_at = now;
                self.into()
            }
            ReaderResult::Died(err) => {
                if let ReaderError::DecodeError(HeartbeatResponseDecodeError) = err {
                    trace!("Heartbeat response doesn't match");
                } else {
                    error!("failed to read() heartbeat response: {err:?}");
                }
                self.disconnect(now)
            }
        }
    }
}

not_supported!(ReadMainConn for ReadingHeartbeatResponse);
not_supported!(WriteMainConn for ReadingHeartbeatResponse);
not_supported!(WriteHeartbeat for ReadingHeartbeatResponse);
