use crate::{
    config::Config,
    connection::{
        ConnectionState, ConnectionWants, ConnectionWantsTo, Disconnect, HasName, Tick,
        WriteMainConn, disconnected::Disconnected, not_supported,
        writing_handshake_request::WritingHandshakeRequest,
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
}

impl HasName for Connecting {
    fn name(&self) -> &'static str {
        "Connecting"
    }
}

impl Disconnect for Connecting {
    fn disconnect(self, now: u64) -> ConnectionState {
        unsafe { rustix::io::close(self.fd.as_raw_fd()) };
        Disconnected::new(now).into()
    }
}

impl Tick for Connecting {
    fn tick(self, now: u64, _config: &Config) -> ConnectionState {
        if now - self.last_activity_at > Self::FREEZE_TIME_IN_SECS {
            self.disconnect(now).into()
        } else {
            self.into()
        }
    }
}

impl ConnectionWantsTo for Connecting {
    fn wants(&self) -> ConnectionWants {
        ConnectionWants {
            conn: Some((self.fd, Wants::Write)),
            heartbeat: None,
        }
    }
}

impl WriteMainConn for Connecting {
    fn write_main_conn(self, now: u64, config: &Config) -> ConnectionState {
        match rustix::net::sockopt::socket_error(self.fd) {
            Ok(Ok(())) => WritingHandshakeRequest::new(self.fd, now, config).into(),
            Ok(Err(err)) | Err(err) => {
                error!("socket_error returned error: {err:?}");
                self.disconnect(now)
            }
        }
    }
}

not_supported!(ReadMainConn for Connecting);
not_supported!(ReadHeartbeat for Connecting);
not_supported!(WriteHeartbeat for Connecting);
