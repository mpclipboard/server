use crate::{Connectivity, config::Config};
use anyhow::Result;
use mpclipboard_shared::{error, event_loop::Wants, info, message::Message};
use std::os::fd::BorrowedFd;

mod helpers;

mod stream;
use stream::Stream;

mod disconnected;
use disconnected::Disconnected;

mod connecting;
use connecting::Connecting;

mod tls_handshake;
use tls_handshake::TlsHandshake;

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
    TlsHandshake(TlsHandshake),
    WritingHandshakeRequest(WritingHandshakeRequest),
    ReadingHandshakeResponse(ReadingHandshakeResponse),
    Connected(Connected),
}

impl ConnectionState {
    fn name(&self) -> &'static str {
        match self {
            Self::Disconnected(_) => "Disconnected",
            Self::Connecting(_) => "Connecting",
            Self::TlsHandshake(_) => "TlsHandshake",
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
impl_connection_state_from!(TlsHandshake);
impl_connection_state_from!(WritingHandshakeRequest);
impl_connection_state_from!(ReadingHandshakeResponse);
impl_connection_state_from!(Connected);

#[derive(Debug)]
pub struct Connection {
    state: ConnectionState,
    config: Config,
    stream: Stream,
}

impl Connection {
    pub(crate) fn new(config: Config) -> Result<Self> {
        Ok(Self {
            state: ConnectionState::Disconnected(Disconnected::new(0)),
            config,
            stream: Stream::empty(),
        })
    }

    pub(crate) fn tick(&mut self, now: u64) {
        match self.state {
            ConnectionState::Disconnected(s) => {
                let (next, stream) = s.try_reconnect(now, &self.config);
                self.stream = stream;
                self.transition(next);
            }
            ConnectionState::Connecting(s) => {
                self.transition(s.disconnect_if_stuck(now));
            }
            ConnectionState::TlsHandshake(s) => {
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
            ConnectionState::TlsHandshake(s) => {
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
            ConnectionState::TlsHandshake(s) => {
                self.with_stream(|stream, config| (s.advance(now, config, stream), None))
            }
            ConnectionState::ReadingHandshakeResponse(s) => {
                self.with_stream(|stream, _config| s.read(now, stream))
            }
            ConnectionState::Connected(s) => {
                self.with_stream(|stream, _config| s.read(now, stream))
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
                self.transition(s.finish(now, &self.config, &self.stream));
            }
            ConnectionState::TlsHandshake(s) => {
                self.with_stream(|stream, config| (s.advance(now, config, stream), None));
            }
            ConnectionState::WritingHandshakeRequest(s) => {
                self.with_stream(|stream, config| (s.write(now, config, stream), None));
            }
            ConnectionState::Connected(s) => {
                self.with_stream(|stream, _config| (s.write(now, stream), None));
            }

            ConnectionState::ReadingHandshakeResponse(s) => {
                self.with_stream(|stream, _config| match stream.flush(&s.fd()) {
                    Ok(()) => (s.into(), None),
                    Err(err) => {
                        error!("failed to flush TLS data: {err:?}");
                        (s.disconnect(now), None)
                    }
                });
            }

            ConnectionState::Disconnected(_) => {
                unreachable!("can't write() in {} state", self.state.name())
            }
        }
    }

    pub(crate) fn wants(&self) -> Option<(BorrowedFd<'static>, Wants)> {
        match self.state {
            ConnectionState::Disconnected(s) => s.wants(),
            ConnectionState::Connecting(s) => s.wants(),
            ConnectionState::TlsHandshake(s) => s.wants(&self.stream),
            ConnectionState::WritingHandshakeRequest(s) => s.wants(&self.stream),
            ConnectionState::ReadingHandshakeResponse(s) => s.wants(&self.stream),
            ConnectionState::Connected(s) => s.wants(&self.stream),
        }
    }

    fn transition(&mut self, next: ConnectionState) {
        let prev = self.state;
        self.state = next;

        if matches!(next, ConnectionState::Disconnected(_)) {
            self.stream = Stream::empty();
        }

        if prev.name() != next.name() {
            info!("Transitioning {} -> {}", prev.name(), next.name())
        }
    }

    pub(crate) fn connectivity(&self) -> Connectivity {
        self.state.connectivity()
    }

    fn with_stream(
        &mut self,
        f: impl FnOnce(&mut Stream, &Config) -> (ConnectionState, Option<Message>),
    ) -> Option<Message> {
        let (next, message) = f(&mut self.stream, &self.config);
        self.transition(next);
        message
    }
}

const FREEZE_TIME_IN_SECS: u64 = 3;
