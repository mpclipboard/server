use crate::connection::{
    ConnectionState, FREEZE_TIME_IN_SECS,
    disconnected::Disconnected,
    stream::{Stream, TlsHandshakeResult},
    writing_handshake_request::WritingHandshakeRequest,
};
use mpclipboard_shared::{error, event_loop::Wants};
use std::os::fd::{AsRawFd, BorrowedFd};

#[derive(Debug, Clone, Copy)]
pub(crate) struct TlsHandshake {
    fd: BorrowedFd<'static>,
    last_activity_at: u64,
}

impl TlsHandshake {
    pub(crate) fn new(fd: BorrowedFd<'static>, now: u64) -> Self {
        Self {
            fd,
            last_activity_at: now,
        }
    }

    pub(crate) fn wants(&self, stream: &Stream) -> Option<(BorrowedFd<'static>, Wants)> {
        Some((self.fd, stream.tls_wants()))
    }

    pub(crate) fn disconnect_if_stuck(self, now: u64) -> ConnectionState {
        if now - self.last_activity_at > FREEZE_TIME_IN_SECS {
            error!("Stuck in TlsHandshake, disconnecting...");
            self.disconnect(now).into()
        } else {
            self.into()
        }
    }

    pub(crate) fn disconnect(self, now: u64) -> ConnectionState {
        unsafe { rustix::io::close(self.fd.as_raw_fd()) };
        Disconnected::new(now).into()
    }

    pub(crate) fn advance(
        mut self,
        now: u64,
        config: &crate::config::Config,
        stream: &mut Stream,
    ) -> ConnectionState {
        match stream.tls_handshake(&self.fd) {
            TlsHandshakeResult::Done => WritingHandshakeRequest::new(self.fd, now, config).into(),
            TlsHandshakeResult::Pending => {
                self.last_activity_at = now;
                self.into()
            }
            TlsHandshakeResult::Died(err) => {
                error!("TLS handshake failed: {err:?}");
                self.disconnect(now)
            }
        }
    }
}
