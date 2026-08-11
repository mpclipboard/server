use crate::{
    config::Config,
    connection::{
        ConnectionState, ConnectionWants, ConnectionWantsTo, Disconnect, HasName, Tick,
        WriteMainConn, disconnected::Disconnected, not_supported,
        reading_handshake_response::ReadingHandshakeResponse,
    },
};
use mpclipboard_shared::{
    error,
    event_loop::Wants,
    messaging::handshake::HandshakeRequest,
    writer::{Writer, WriterResult},
};
use std::os::fd::{AsRawFd, BorrowedFd};

#[derive(Debug, Clone, Copy)]
pub(crate) struct WritingHandshakeRequest {
    fd: BorrowedFd<'static>,
    writer: Writer<{ HandshakeRequest::BYTESIZE }, HandshakeRequest>,
    last_activity_at: u64,
}

impl WritingHandshakeRequest {
    pub(crate) fn new(fd: BorrowedFd<'static>, now: u64, config: &Config) -> Self {
        let handshake_req = HandshakeRequest {
            host: config.main_url.header(),
            token: config.token,
            id: config.id,
        };

        Self {
            fd,
            writer: Writer::new(&handshake_req),
            last_activity_at: now,
        }
    }
}

impl HasName for WritingHandshakeRequest {
    fn name(&self) -> &'static str {
        "WritingHandshakeRequest"
    }
}

impl Disconnect for WritingHandshakeRequest {
    fn disconnect(self, now: u64) -> ConnectionState {
        unsafe { rustix::io::close(self.fd.as_raw_fd()) };
        Disconnected::new(now).into()
    }
}

impl Tick for WritingHandshakeRequest {
    fn tick(self, now: u64, _config: &Config) -> ConnectionState {
        if now - self.last_activity_at > Self::FREEZE_TIME_IN_SECS {
            self.disconnect(now).into()
        } else {
            self.into()
        }
    }
}

impl ConnectionWantsTo for WritingHandshakeRequest {
    fn wants(&self) -> ConnectionWants {
        ConnectionWants {
            conn: Some((self.fd, Wants::Write)),
            heartbeat: None,
        }
    }
}

impl WriteMainConn for WritingHandshakeRequest {
    fn write_main_conn(mut self, now: u64, _config: &Config) -> ConnectionState {
        match self.writer.write(&self.fd) {
            WriterResult::Done => ReadingHandshakeResponse::new(self.fd, now).into(),
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

not_supported!(ReadMainConn for WritingHandshakeRequest);
not_supported!(ReadHeartbeat for WritingHandshakeRequest);
not_supported!(WriteHeartbeat for WritingHandshakeRequest);
