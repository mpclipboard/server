use crate::{
    config::Config,
    connection::{
        ConnectionState, FREEZE_TIME_IN_SECS, disconnected::Disconnected,
        reading_handshake_response::ReadingHandshakeResponse,
    },
};
use mpclipboard_shared::{
    error,
    event_loop::Wants,
    messaging::handshake::request::HandshakeRequest,
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
            host: config.url.header(),
            token: config.token,
            id: config.id,
        };

        Self {
            fd,
            writer: Writer::new(&handshake_req),
            last_activity_at: now,
        }
    }

    pub(crate) fn wants(&self) -> Option<(BorrowedFd<'static>, Wants)> {
        Some((self.fd, Wants::Write))
    }

    pub(crate) fn disconnect_if_stuck(self, now: u64) -> ConnectionState {
        if now - self.last_activity_at > FREEZE_TIME_IN_SECS {
            self.disconnect(now).into()
        } else {
            self.into()
        }
    }

    pub(crate) fn disconnect(self, now: u64) -> ConnectionState {
        unsafe { rustix::io::close(self.fd.as_raw_fd()) };
        Disconnected::new(now).into()
    }

    pub(crate) fn write(mut self, now: u64, _config: &Config) -> ConnectionState {
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
