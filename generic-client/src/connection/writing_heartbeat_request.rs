use crate::{
    config::Config,
    connection::{
        ConnectionState, ConnectionWants, ConnectionWantsTo, Disconnect, HasName, Tick,
        WriteHeartbeat, disconnected::Disconnected, not_supported,
        reading_heartbeat_response::ReadingHeartbeatResponse,
    },
};
use mpclipboard_shared::{
    error,
    event_loop::Wants,
    heartbeat::HeartbeatRequest,
    writer::{Writer, WriterResult},
};
use std::os::fd::{AsRawFd, BorrowedFd};

#[derive(Debug, Clone, Copy)]
pub struct WritingHeartbeatRequest {
    connfd: BorrowedFd<'static>,
    heartbeatfd: BorrowedFd<'static>,
    writer: Writer<{ HeartbeatRequest::BYTESIZE }, HeartbeatRequest>,
    last_activity_at: u64,
}

impl WritingHeartbeatRequest {
    pub(crate) fn new(
        connfd: BorrowedFd<'static>,
        heartbeatfd: BorrowedFd<'static>,
        now: u64,
        config: &Config,
    ) -> Self {
        let hearbeat_req = HeartbeatRequest {
            host: config.heartbeat_url.header(),
            id: config.id,
        };

        Self {
            connfd,
            heartbeatfd,
            writer: Writer::new(&hearbeat_req),
            last_activity_at: now,
        }
    }
}

impl HasName for WritingHeartbeatRequest {
    fn name(&self) -> &'static str {
        "WritingHeartbeatRequest"
    }
}

impl Tick for WritingHeartbeatRequest {
    fn tick(self, now: u64, _config: &Config) -> ConnectionState {
        if now - self.last_activity_at > Self::FREEZE_TIME_IN_SECS {
            self.disconnect(now).into()
        } else {
            self.into()
        }
    }
}

impl Disconnect for WritingHeartbeatRequest {
    fn disconnect(self, now: u64) -> ConnectionState {
        unsafe { rustix::io::close(self.connfd.as_raw_fd()) };
        unsafe { rustix::io::close(self.heartbeatfd.as_raw_fd()) };
        Disconnected::new(now).into()
    }
}

impl ConnectionWantsTo for WritingHeartbeatRequest {
    fn wants(&self) -> ConnectionWants {
        ConnectionWants {
            conn: None,
            heartbeat: Some((self.heartbeatfd, Wants::Write)),
        }
    }
}

impl WriteHeartbeat for WritingHeartbeatRequest {
    fn write_heartbeat(mut self, now: u64, _config: &Config) -> ConnectionState {
        match self.writer.write(&self.heartbeatfd) {
            WriterResult::Done => {
                ReadingHeartbeatResponse::new(self.connfd, self.heartbeatfd, now).into()
            }
            WriterResult::StillPending => {
                self.last_activity_at = now;
                self.into()
            }
            WriterResult::Died(err) => {
                error!("write() failed: {err:?}");
                self.disconnect(now)
            }
        }
    }
}

not_supported!(ReadMainConn for WritingHeartbeatRequest);
not_supported!(WriteMainConn for WritingHeartbeatRequest);
not_supported!(ReadHeartbeat for WritingHeartbeatRequest);
