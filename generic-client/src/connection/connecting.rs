use crate::{
    config::Config,
    connection::{
        ConnectionState, FREEZE_TIME_IN_SECS, disconnected::Disconnected,
        maybe_tls_stream::MaybeTlsStream, tls_handshake::TlsHandshake,
        writing_handshake_request::WritingHandshakeRequest,
    },
};
use mpclipboard_shared::{Wants, error};
use std::os::fd::{AsRawFd, BorrowedFd};

#[derive(Debug, Clone, Copy)]
pub struct Connecting {
    fd: BorrowedFd<'static>,
    last_activity_at: u64,
}

impl Connecting {
    pub(crate) const fn new(fd: BorrowedFd<'static>, now: u64) -> Self {
        Self {
            fd,
            last_activity_at: now,
        }
    }

    pub(crate) const fn wants(&self) -> (BorrowedFd<'static>, Wants) {
        (self.fd, Wants::Write)
    }

    pub(crate) fn disconnect_if_stuck(self, now: u64) -> ConnectionState {
        let diff = now
            .checked_sub(self.last_activity_at)
            .unwrap_or_else(|| unreachable!("time goes backwards"));
        if diff > FREEZE_TIME_IN_SECS {
            error!("Stuck in Connecting, disconnecting...");
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
        self,
        now: u64,
        config: &Config,
        stream: &MaybeTlsStream,
    ) -> ConnectionState {
        match rustix::net::sockopt::socket_error(self.fd) {
            Ok(Ok(())) => {
                if stream.is_tls() {
                    TlsHandshake::new(self.fd, now).into()
                } else {
                    WritingHandshakeRequest::new(self.fd, now, config).into()
                }
            }
            Ok(Err(err)) | Err(err) => {
                error!("socket_error returned error: {err:?}");
                self.disconnect(now)
            }
        }
    }
}
