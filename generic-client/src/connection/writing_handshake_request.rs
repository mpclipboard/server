use crate::{
    config::Config,
    connection::{
        ConnectionState, FREEZE_TIME_IN_SECS, disconnected::Disconnected,
        maybe_tls_stream::MaybeTlsStream, reading_handshake_response::ReadingHandshakeResponse,
    },
};
use mpclipboard_shared::{HandshakeRequest, HandshakeRequestWriter, Wants, error};
use std::os::fd::{AsRawFd, BorrowedFd};

#[derive(Debug, Clone, Copy)]
pub(crate) struct WritingHandshakeRequest {
    fd: BorrowedFd<'static>,
    writer: HandshakeRequestWriter,
    last_activity_at: u64,
}

impl WritingHandshakeRequest {
    pub(crate) fn new(fd: BorrowedFd<'static>, now: u64, config: &Config) -> Self {
        let handshake_req = HandshakeRequest {
            host: config.url.header(),
            token: config.token,
            id: config.id,
        };

        Self {
            fd,
            writer: HandshakeRequestWriter::new(&handshake_req),
            last_activity_at: now,
        }
    }

    pub(crate) fn wants(&self, stream: &MaybeTlsStream) -> Option<(BorrowedFd<'static>, Wants)> {
        Some((self.fd, stream.wants(Wants::Write)))
    }

    pub(crate) fn disconnect_if_stuck(self, now: u64) -> ConnectionState {
        if now - self.last_activity_at > FREEZE_TIME_IN_SECS {
            error!("Stuck in WritingHandshakeRequest, disconnecting...");
            self.disconnect(now).into()
        } else {
            self.into()
        }
    }

    pub(crate) fn disconnect(self, now: u64) -> ConnectionState {
        unsafe { rustix::io::close(self.fd.as_raw_fd()) };
        Disconnected::new(now).into()
    }

    pub(crate) fn write(
        mut self,
        now: u64,
        _config: &Config,
        stream: &mut MaybeTlsStream,
    ) -> ConnectionState {
        match self.writer.write_to(stream, &self.fd) {
            Ok(true) => ReadingHandshakeResponse::new(self.fd, now).into(),
            Ok(false) => {
                self.last_activity_at = now;
                self.into()
            }
            Err(err) => {
                error!("write() failed: {err:?}");
                self.disconnect(now)
            }
        }
    }
}
