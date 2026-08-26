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
pub struct WritingHandshakeRequest {
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

    pub(crate) fn wants(&self, stream: &MaybeTlsStream) -> (BorrowedFd<'static>, Wants) {
        (self.fd, stream.wants(Wants::Write))
    }

    pub(crate) fn disconnect_if_stuck(self, now: u64) -> ConnectionState {
        let diff = now
            .checked_sub(self.last_activity_at)
            .unwrap_or_else(|| unreachable!("time goes backwards"));
        if diff > FREEZE_TIME_IN_SECS {
            error!("Stuck in WritingHandshakeRequest, disconnecting...");
            self.disconnect(now)
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
        match stream.write_bytes(&self.fd, self.writer.remainder()) {
            Ok(Some(len)) => {
                if self.writer.written(len) {
                    ReadingHandshakeResponse::new(self.fd, now).into()
                } else {
                    self.last_activity_at = now;
                    self.into()
                }
            }
            Ok(None) => {
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
