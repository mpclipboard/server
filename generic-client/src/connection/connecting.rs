use crate::{
    config::Config,
    connection::{
        ConnectionState, FREEZE_TIME_IN_SECS, disconnected::Disconnected, stream::Stream,
        tls_handshake::TlsHandshake, writing_handshake_request::WritingHandshakeRequest,
    },
};
use mpclipboard_shared::{error, event_loop::Wants};
use std::os::fd::{AsRawFd, BorrowedFd};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Connecting {
    fd: BorrowedFd<'static>,
    last_activity_at: u64,
}

impl Connecting {
    pub(crate) fn new(fd: BorrowedFd<'static>, now: u64) -> Self {
        Self {
            fd,
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

    pub(crate) fn finish(self, now: u64, config: &Config, stream: &Stream) -> ConnectionState {
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
