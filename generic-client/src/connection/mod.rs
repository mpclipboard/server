use crate::{Connectivity, config::Config};
use anyhow::Result;
use mpclipboard_shared::{event_loop::Wants, info, messaging::message::Message};
use std::os::fd::BorrowedFd;

mod helpers;

mod disconnected;
use disconnected::Disconnected;

mod connecting;
use connecting::Connecting;

mod writing_handshake_request;
use writing_handshake_request::WritingHandshakeRequest;

mod reading_handshake_response;
use reading_handshake_response::ReadingHandshakeResponse;

mod connecting_to_heartbeat;
use connecting_to_heartbeat::ConnectingToHeartbeat;

mod writing_heartbeat_request;
use writing_heartbeat_request::WritingHeartbeatRequest;

mod reading_heartbeat_response;
use reading_heartbeat_response::ReadingHeartbeatResponse;

mod connected;
use connected::Connected;

#[derive(Debug, Clone, Copy)]
pub(crate) enum ConnectionState {
    Disconnected(Disconnected),
    Connecting(Connecting),
    WritingHandshakeRequest(WritingHandshakeRequest),
    ReadingHandshakeResponse(ReadingHandshakeResponse),
    ConnectingToHeartbeat(ConnectingToHeartbeat),
    WritingHeartbeatRequest(WritingHeartbeatRequest),
    ReadingHeartbeatResponse(ReadingHeartbeatResponse),
    Connected(Connected),
}

macro_rules! for_each_connection_state {
    ($value:expr => |$var:ident| $eval:expr) => {
        match $value {
            ConnectionState::Disconnected($var) => $eval,
            ConnectionState::Connecting($var) => $eval,
            ConnectionState::WritingHandshakeRequest($var) => $eval,
            ConnectionState::ReadingHandshakeResponse($var) => $eval,
            ConnectionState::ConnectingToHeartbeat($var) => $eval,
            ConnectionState::WritingHeartbeatRequest($var) => $eval,
            ConnectionState::ReadingHeartbeatResponse($var) => $eval,
            ConnectionState::Connected($var) => $eval,
        }
    };
}

impl ConnectionState {
    fn name(&self) -> &'static str {
        for_each_connection_state!(self => |s| s.name())
    }

    const fn connectivity(&self) -> Connectivity {
        match self {
            Self::Disconnected(_) => Connectivity::Disconnected,
            Self::Connected(_) => Connectivity::Connected,
            _ => Connectivity::Connecting,
        }
    }
}

macro_rules! impl_connection_state_from {
    ($t:ident) => {
        impl From<$t> for ConnectionState {
            fn from(s: $t) -> Self {
                Self::$t(s)
            }
        }
    };
}
impl_connection_state_from!(Disconnected);
impl_connection_state_from!(Connecting);
impl_connection_state_from!(WritingHandshakeRequest);
impl_connection_state_from!(ReadingHandshakeResponse);
impl_connection_state_from!(ConnectingToHeartbeat);
impl_connection_state_from!(WritingHeartbeatRequest);
impl_connection_state_from!(ReadingHeartbeatResponse);
impl_connection_state_from!(Connected);

#[derive(Debug)]
pub struct Connection {
    state: ConnectionState,
    config: Config,
}

impl Connection {
    pub(crate) fn new(config: Config) -> Result<Self> {
        Ok(Self {
            state: ConnectionState::Disconnected(Disconnected::new(0)),
            config,
        })
    }

    pub(crate) fn tick(&mut self, now: u64) {
        for_each_connection_state!(self.state => |s| {
            self.transition(s.tick(now, &self.config));
        });
    }

    pub(crate) fn push(&mut self, message: Message) -> bool {
        if let ConnectionState::Connected(s) = &mut self.state {
            s.push(message);
            true
        } else {
            false
        }
    }

    pub(crate) fn disconnect(&mut self, now: u64) {
        for_each_connection_state!(self.state => |s| {
            self.transition(s.disconnect(now));
        });
    }

    pub(crate) fn is_disconnected(&self) -> bool {
        matches!(self.state, ConnectionState::Disconnected(_))
    }

    pub(crate) fn on_main_conn_readable(&mut self, now: u64) -> Option<Message> {
        for_each_connection_state!(self.state => |s| {
            let (next, incoming) = s.read_main_conn(now, &self.config);
            self.transition(next);
            incoming
        })
    }

    pub(crate) fn on_main_conn_writable(&mut self, now: u64) {
        for_each_connection_state!(self.state => |s| {
            self.transition(s.write_main_conn(now, &self.config));
        });
    }

    pub(crate) fn on_heartbeat_readable(&mut self, now: u64) {
        for_each_connection_state!(self.state => |s| {
            self.transition(s.read_heartbeat(now));
        });
    }

    pub(crate) fn on_heartbeat_writable(&mut self, now: u64) {
        for_each_connection_state!(self.state => |s| {
            self.transition(s.write_heartbeat(now, &self.config));
        });
    }

    pub(crate) fn wants(&self) -> ConnectionWants {
        for_each_connection_state!(self.state => |s| {
            s.wants()
        })
    }

    fn transition(&mut self, next: ConnectionState) {
        let prev = self.state;
        self.state = next;

        if prev.name() != next.name() {
            info!("Transitioning {} -> {}", prev.name(), next.name())
        }
    }

    pub(crate) fn connectivity(&self) -> Connectivity {
        self.state.connectivity()
    }
}

#[derive(Debug)]
pub struct ConnectionWants {
    pub conn: Option<(BorrowedFd<'static>, Wants)>,
    pub heartbeat: Option<(BorrowedFd<'static>, Wants)>,
}
impl ConnectionWants {
    pub(crate) fn nothing() -> Self {
        Self {
            conn: None,
            heartbeat: None,
        }
    }
}

trait HasName {
    fn name(&self) -> &'static str;
}

trait Disconnect {
    fn disconnect(self, now: u64) -> ConnectionState;
}

trait ReadMainConn {
    fn read_main_conn(self, _now: u64, _config: &Config) -> (ConnectionState, Option<Message>);
}

trait WriteMainConn {
    fn write_main_conn(self, _now: u64, _config: &Config) -> ConnectionState;
}

trait ReadHeartbeat {
    fn read_heartbeat(self, _now: u64) -> ConnectionState;
}

trait WriteHeartbeat {
    fn write_heartbeat(self, _now: u64, _config: &Config) -> ConnectionState;
}

trait Tick {
    const FREEZE_TIME_IN_SECS: u64 = 3;

    fn tick(self, _now: u64, _config: &Config) -> ConnectionState;
}

trait ConnectionWantsTo {
    fn wants(&self) -> ConnectionWants;
}

macro_rules! not_supported {
    (ReadMainConn for $t:ty) => {
        impl $crate::connection::ReadMainConn for $t {
            fn read_main_conn(
                self,
                _now: u64,
                _config: &$crate::config::Config,
            ) -> (
                $crate::connection::ConnectionState,
                Option<mpclipboard_shared::messaging::message::Message>,
            ) {
                unreachable!("can't read() from main connection in {}", stringify!($ty))
            }
        }
    };

    (WriteMainConn for $t:ty) => {
        impl $crate::connection::WriteMainConn for $t {
            fn write_main_conn(
                self,
                _now: u64,
                _config: &$crate::config::Config,
            ) -> $crate::connection::ConnectionState {
                unreachable!("can't write() from main connection in {}", stringify!($ty))
            }
        }
    };

    (ReadHeartbeat for $t:ty) => {
        impl $crate::connection::ReadHeartbeat for $t {
            fn read_heartbeat(self, _now: u64) -> $crate::connection::ConnectionState {
                unreachable!(
                    "can't read_heartbeat() from main connection in {}",
                    stringify!($ty)
                )
            }
        }
    };

    (WriteHeartbeat for $t:ty) => {
        impl $crate::connection::WriteHeartbeat for $t {
            fn write_heartbeat(
                self,
                _now: u64,
                _config: &$crate::config::Config,
            ) -> $crate::connection::ConnectionState {
                unreachable!(
                    "can't write_heartbeat() from main connection in {}",
                    stringify!($ty)
                )
            }
        }
    };
}
pub(crate) use not_supported;
