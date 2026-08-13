use crate::{
    config::Config,
    connection::{
        ConnectionState,
        connecting::Connecting,
        helpers::{ConnectResult, connect},
        maybe_tls_stream::MaybeTlsStream,
        tls_handshake::TlsHandshake,
        writing_handshake_request::WritingHandshakeRequest,
    },
};
use anyhow::Context;
use mpclipboard_shared::error;
use std::os::fd::AsRawFd;

#[derive(Debug, Clone, Copy)]
pub struct Disconnected {
    last_activity_at: u64,
}

impl Disconnected {
    const RECONNECT_AFTER: u64 = 3;

    pub(crate) const fn new(now: u64) -> Self {
        Self {
            last_activity_at: now,
        }
    }

    pub(crate) fn try_reconnect(
        mut self,
        now: u64,
        config: &Config,
    ) -> (ConnectionState, MaybeTlsStream) {
        let diff = now
            .checked_sub(self.last_activity_at)
            .unwrap_or_else(|| unreachable!("time goes backwards"));
        if diff < Self::RECONNECT_AFTER {
            return (self.into(), MaybeTlsStream::empty());
        }

        let addr = match config.url.resolve() {
            Ok(addr) => addr,
            Err(err) => {
                error!("failed to get IP address of the url: {err:?}");
                self.last_activity_at = now;
                return (self.into(), MaybeTlsStream::empty());
            }
        };

        let (fd, connected_now) = match connect(addr) {
            ConnectResult::Connected(fd) => (fd, true),
            ConnectResult::StillPending(fd) => (fd, false),
            ConnectResult::Failed => {
                self.last_activity_at = now;
                return (self.into(), MaybeTlsStream::empty());
            }
        };

        let stream = match MaybeTlsStream::new(&config.url).context("failed to create stream") {
            Ok(stream) => stream,
            Err(err) => {
                error!("{err:?}");
                unsafe { rustix::io::close(fd.as_raw_fd()) };
                self.last_activity_at = now;
                return (self.into(), MaybeTlsStream::empty());
            }
        };

        let state = if connected_now {
            if stream.is_tls() {
                TlsHandshake::new(fd, now).into()
            } else {
                WritingHandshakeRequest::new(fd, now, config).into()
            }
        } else {
            Connecting::new(fd, now).into()
        };

        (state, stream)
    }
}
