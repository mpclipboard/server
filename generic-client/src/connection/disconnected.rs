use crate::{
    config::Config,
    connection::{
        ConnectionState,
        connecting::Connecting,
        helpers::{ConnectResult, connect},
        writing_handshake_request::WritingHandshakeRequest,
    },
};
use mpclipboard_shared::{error, event_loop::Wants};
use std::os::fd::BorrowedFd;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Disconnected {
    last_activity_at: u64,
}

impl Disconnected {
    const RECONNECT_AFTER: u64 = 3;

    pub fn new(now: u64) -> Self {
        Self {
            last_activity_at: now,
        }
    }

    pub(crate) fn wants(&self) -> Option<(BorrowedFd<'static>, Wants)> {
        None
    }

    pub(crate) fn try_reconnect(self, now: u64, config: &Config) -> ConnectionState {
        if now - self.last_activity_at >= Self::RECONNECT_AFTER {
            self.reconnect(now, config)
        } else {
            self.into()
        }
    }

    fn reconnect(&self, now: u64, config: &Config) -> ConnectionState {
        let addr = match config.url.resolve() {
            Ok(addr) => addr,
            Err(err) => {
                error!("failed to get IP address of the url: {err:?}");
                return Self::new(now).into();
            }
        };

        match connect(&addr) {
            ConnectResult::Connected(fd) => WritingHandshakeRequest::new(fd, now, config).into(),
            ConnectResult::StillPending(fd) => Connecting::new(fd, now).into(),
            ConnectResult::Failed => Self::new(now).into(),
        }
    }
}
