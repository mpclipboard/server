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

mod connected;
use connected::Connected;

#[derive(Debug, Clone, Copy)]
pub(crate) enum ConnectionState {
    Disconnected(Disconnected),
    Connecting(Connecting),
    WritingHandshakeRequest(WritingHandshakeRequest),
    ReadingHandshakeResponse(ReadingHandshakeResponse),
    Connected(Connected),
}

impl ConnectionState {
    fn name(&self) -> &'static str {
        match self {
            Self::Disconnected(_) => "Disconnected",
            Self::Connecting(_) => "Connecting",
            Self::WritingHandshakeRequest(_) => "WritingHandshakeRequest",
            Self::ReadingHandshakeResponse(_) => "ReadingHandshakeResponse",
            Self::Connected(_) => "Connected",
        }
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
        match self.state {
            ConnectionState::Disconnected(s) => {
                self.transition(s.try_reconnect(now, &self.config));
            }
            ConnectionState::Connecting(s) => {
                self.transition(s.disconnect_if_stuck(now));
            }
            ConnectionState::WritingHandshakeRequest(s) => {
                self.transition(s.disconnect_if_stuck(now));
            }
            ConnectionState::ReadingHandshakeResponse(s) => {
                self.transition(s.disconnect_if_stuck(now));
            }
            ConnectionState::Connected(_) => {}
        }
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
        match self.state {
            ConnectionState::Disconnected(_) => {
                unreachable!("can't disconnect() in Disconnected state");
            }
            ConnectionState::Connecting(s) => {
                self.transition(s.disconnect(now));
            }
            ConnectionState::WritingHandshakeRequest(s) => {
                self.transition(s.disconnect(now));
            }
            ConnectionState::ReadingHandshakeResponse(s) => {
                self.transition(s.disconnect(now));
            }
            ConnectionState::Connected(s) => {
                self.transition(s.disconnect(now));
            }
        }
    }

    pub(crate) fn is_disconnected(&self) -> bool {
        matches!(self.state, ConnectionState::Disconnected(_))
    }

    pub(crate) fn on_readable(&mut self, now: u64) -> Option<Message> {
        match self.state {
            ConnectionState::ReadingHandshakeResponse(s) => {
                self.transition(s.read(now));
                None
            }
            ConnectionState::Connected(s) => {
                let (next, incoming) = s.read(now);
                self.transition(next);
                incoming
            }

            ConnectionState::Disconnected(_)
            | ConnectionState::Connecting(_)
            | ConnectionState::WritingHandshakeRequest(_) => {
                unreachable!("can't read() in {} state", self.state.name())
            }
        }
    }

    pub(crate) fn on_writable(&mut self, now: u64) {
        match self.state {
            ConnectionState::Connecting(s) => {
                self.transition(s.finish(now, &self.config));
            }
            ConnectionState::WritingHandshakeRequest(s) => {
                self.transition(s.write(now, &self.config));
            }
            ConnectionState::Connected(s) => {
                self.transition(s.write(now));
            }

            ConnectionState::ReadingHandshakeResponse(_) | ConnectionState::Disconnected(_) => {
                unreachable!("can't write() in {} state", self.state.name())
            }
        }
    }

    pub(crate) fn wants(&self) -> Option<(BorrowedFd<'static>, Wants)> {
        match self.state {
            ConnectionState::Disconnected(s) => s.wants(),
            ConnectionState::Connecting(s) => s.wants(),
            ConnectionState::WritingHandshakeRequest(s) => s.wants(),
            ConnectionState::ReadingHandshakeResponse(s) => s.wants(),
            ConnectionState::Connected(s) => s.wants(),
        }
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

const FREEZE_TIME_IN_SECS: u64 = 3;
