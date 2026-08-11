use crate::{
    config::Config,
    connection::{
        ConnectionState, ConnectionWants, ConnectionWantsTo, Disconnect, HasName, Tick,
        WriteHeartbeat, disconnected::Disconnected, not_supported,
        writing_heartbeat_request::WritingHeartbeatRequest,
    },
};
use mpclipboard_shared::{error, event_loop::Wants};
use std::os::fd::{AsRawFd, BorrowedFd};

#[derive(Debug, Clone, Copy)]
pub struct ConnectingToHeartbeat {
    connfd: BorrowedFd<'static>,
    heartbeatfd: BorrowedFd<'static>,
    last_activity_at: u64,
}

impl ConnectingToHeartbeat {
    pub(crate) fn new(
        connfd: BorrowedFd<'static>,
        heartbeatfd: BorrowedFd<'static>,
        now: u64,
    ) -> Self {
        Self {
            connfd,
            heartbeatfd,
            last_activity_at: now,
        }
    }
}

impl HasName for ConnectingToHeartbeat {
    fn name(&self) -> &'static str {
        "ConnectingToHeartbeat"
    }
}

impl Tick for ConnectingToHeartbeat {
    fn tick(self, now: u64, _config: &Config) -> ConnectionState {
        if now - self.last_activity_at > Self::FREEZE_TIME_IN_SECS {
            self.disconnect(now).into()
        } else {
            self.into()
        }
    }
}

impl Disconnect for ConnectingToHeartbeat {
    fn disconnect(self, now: u64) -> ConnectionState {
        unsafe { rustix::io::close(self.connfd.as_raw_fd()) };
        unsafe { rustix::io::close(self.heartbeatfd.as_raw_fd()) };
        Disconnected::new(now).into()
    }
}

impl ConnectionWantsTo for ConnectingToHeartbeat {
    fn wants(&self) -> ConnectionWants {
        ConnectionWants {
            conn: None,
            heartbeat: Some((self.heartbeatfd, Wants::Write)),
        }
    }
}

impl WriteHeartbeat for ConnectingToHeartbeat {
    fn write_heartbeat(self, now: u64, config: &Config) -> ConnectionState {
        match rustix::net::sockopt::socket_error(self.heartbeatfd) {
            Ok(Ok(())) => {
                WritingHeartbeatRequest::new(self.connfd, self.heartbeatfd, now, config).into()
            }
            Ok(Err(err)) | Err(err) => {
                error!("socket_error returned error: {err:?}");
                self.disconnect(now)
            }
        }
    }
}

not_supported!(ReadMainConn for ConnectingToHeartbeat);
not_supported!(WriteMainConn for ConnectingToHeartbeat);
not_supported!(ReadHeartbeat for ConnectingToHeartbeat);
