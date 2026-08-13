use crate::{
    config::Config,
    connection::{
        ConnectionState, FREEZE_TIME_IN_SECS,
        disconnected::Disconnected,
        maybe_tls_stream::{MaybeTlsStream, TlsHandshakeResult},
        writing_handshake_request::WritingHandshakeRequest,
    },
};
use mpclipboard_shared::{Wants, error};
use std::os::fd::{AsRawFd, BorrowedFd};

#[derive(Debug, Clone, Copy)]
pub struct TlsHandshake {
    fd: BorrowedFd<'static>,
    last_activity_at: u64,
}

impl TlsHandshake {
    pub(crate) const fn new(fd: BorrowedFd<'static>, now: u64) -> Self {
        Self {
            fd,
            last_activity_at: now,
        }
    }

    pub(crate) fn wants(&self, stream: &MaybeTlsStream) -> (BorrowedFd<'static>, Wants) {
        (self.fd, stream.tls_wants())
    }

    pub(crate) fn disconnect_if_stuck(self, now: u64) -> ConnectionState {
        let diff = now
            .checked_sub(self.last_activity_at)
            .unwrap_or_else(|| unreachable!("time goes backwards"));
        if diff > FREEZE_TIME_IN_SECS {
            error!("Stuck in TlsHandshake, disconnecting...");
            self.disconnect(now)
        } else {
            self.into()
        }
    }

    pub(crate) fn disconnect(self, now: u64) -> ConnectionState {
        unsafe { rustix::io::close(self.fd.as_raw_fd()) };
        Disconnected::new(now).into()
    }

    pub(crate) fn finish(
        mut self,
        now: u64,
        config: &Config,
        stream: &mut MaybeTlsStream,
    ) -> ConnectionState {
        match stream.finish_tls_handshake(&self.fd) {
            TlsHandshakeResult::Done => WritingHandshakeRequest::new(self.fd, now, config).into(),
            TlsHandshakeResult::Pending => {
                self.last_activity_at = now;
                self.into()
            }
            TlsHandshakeResult::Died => {
                error!("TLS handshake failed");
                self.disconnect(now)
            }
        }
    }
}
