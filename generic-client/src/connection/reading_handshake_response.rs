use crate::{
    config::Config,
    connection::{
        ConnectionState, ConnectionWants, ConnectionWantsTo, Disconnect, HasName, ReadMainConn,
        Tick,
        connecting_to_heartbeat::ConnectingToHeartbeat,
        disconnected::Disconnected,
        helpers::{ConnectResult, connect},
        not_supported,
        writing_heartbeat_request::WritingHeartbeatRequest,
    },
};
use mpclipboard_shared::{
    error,
    event_loop::Wants,
    messaging::{
        handshake::{HandshakeResponse, HandshakeResponseDecodeError},
        message::Message,
    },
    reader::{Reader, ReaderError, ReaderResult},
    trace,
};
use std::os::fd::{AsRawFd, BorrowedFd};

#[derive(Debug, Clone, Copy)]
pub(crate) struct ReadingHandshakeResponse {
    fd: BorrowedFd<'static>,
    reader: Reader<{ HandshakeResponse::BYTESIZE }, HandshakeResponse>,
    last_activity_at: u64,
}

impl ReadingHandshakeResponse {
    pub(crate) fn new(fd: BorrowedFd<'static>, now: u64) -> Self {
        Self {
            fd,
            reader: Reader::new(),
            last_activity_at: now,
        }
    }
}

impl HasName for ReadingHandshakeResponse {
    fn name(&self) -> &'static str {
        "ReadingHandshakeResponse"
    }
}

impl Tick for ReadingHandshakeResponse {
    fn tick(self, now: u64, _config: &Config) -> ConnectionState {
        if now - self.last_activity_at > Self::FREEZE_TIME_IN_SECS {
            self.disconnect(now).into()
        } else {
            self.into()
        }
    }
}

impl Disconnect for ReadingHandshakeResponse {
    fn disconnect(self, now: u64) -> ConnectionState {
        unsafe { rustix::io::close(self.fd.as_raw_fd()) };
        Disconnected::new(now).into()
    }
}

impl ConnectionWantsTo for ReadingHandshakeResponse {
    fn wants(&self) -> ConnectionWants {
        ConnectionWants {
            conn: Some((self.fd, Wants::Read)),
            heartbeat: None,
        }
    }
}

impl ReadMainConn for ReadingHandshakeResponse {
    fn read_main_conn(mut self, now: u64, config: &Config) -> (ConnectionState, Option<Message>) {
        match self.reader.read(&self.fd) {
            ReaderResult::Data(_res) => {
                trace!("Handshake response matches");

                let addr = match config.heartbeat_url.resolve() {
                    Ok(addr) => addr,
                    Err(err) => {
                        error!("failed to get IP address of the main-url: {err:?}");
                        return (self.disconnect(now).into(), None);
                    }
                };

                match connect(&addr) {
                    ConnectResult::Connected(fd) => (
                        WritingHeartbeatRequest::new(self.fd, fd, now, config).into(),
                        None,
                    ),
                    ConnectResult::StillPending(fd) => {
                        (ConnectingToHeartbeat::new(self.fd, fd, now).into(), None)
                    }
                    ConnectResult::Failed => (self.disconnect(now).into(), None),
                }
            }
            ReaderResult::StillPending => {
                self.last_activity_at = now;
                (self.into(), None)
            }
            ReaderResult::Died(err) => {
                if let ReaderError::DecodeError(HandshakeResponseDecodeError) = err {
                    trace!("Handshake response doesn't match");
                } else {
                    error!("failed to read() handshake response: {err:?}");
                }
                (self.disconnect(now), None)
            }
        }
    }
}

not_supported!(WriteMainConn for ReadingHandshakeResponse);
not_supported!(ReadHeartbeat for ReadingHandshakeResponse);
not_supported!(WriteHeartbeat for ReadingHandshakeResponse);
