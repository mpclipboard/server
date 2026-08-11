use crate::{
    config::Config,
    connection::{
        ConnectionState, ConnectionWants, ConnectionWantsTo, Disconnect, HasName, Tick,
        connecting::Connecting,
        helpers::{ConnectResult, connect},
        not_supported,
        writing_handshake_request::WritingHandshakeRequest,
    },
};
use mpclipboard_shared::error;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Disconnected {
    last_activity_at: u64,
}

impl Disconnected {
    pub fn new(now: u64) -> Self {
        Self {
            last_activity_at: now,
        }
    }

    pub(crate) fn reconnect(&self, now: u64, config: &Config) -> ConnectionState {
        let addr = match config.main_url.resolve() {
            Ok(addr) => addr,
            Err(err) => {
                error!("failed to get IP address of the main-url: {err:?}");
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

impl HasName for Disconnected {
    fn name(&self) -> &'static str {
        "Disconnected"
    }
}

impl Disconnect for Disconnected {
    fn disconnect(self, _now: u64) -> ConnectionState {
        unreachable!("can't disconnect() in Disconnected state")
    }
}

impl Tick for Disconnected {
    fn tick(self, now: u64, config: &Config) -> ConnectionState {
        if now - self.last_activity_at >= Self::FREEZE_TIME_IN_SECS {
            self.reconnect(now, config)
        } else {
            self.into()
        }
    }
}

impl ConnectionWantsTo for Disconnected {
    fn wants(&self) -> ConnectionWants {
        ConnectionWants::nothing()
    }
}

not_supported!(ReadMainConn for Disconnected);
not_supported!(WriteMainConn for Disconnected);
not_supported!(ReadHeartbeat for Disconnected);
not_supported!(WriteHeartbeat for Disconnected);
