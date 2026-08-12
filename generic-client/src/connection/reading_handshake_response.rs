use crate::connection::{
    ConnectionState, FREEZE_TIME_IN_SECS, connected::Connected, disconnected::Disconnected,
};
use mpclipboard_shared::{
    error,
    event_loop::Wants,
    http_lines_reader::{HttpLinesParser, HttpLinesReader, HttpLinesReaderResult},
    messaging::handshake::response::HandshakeResponseParser,
    tcp_keep_alive::enable_tcp_keep_alive,
    trace,
};
use std::os::fd::{AsRawFd, BorrowedFd};

#[derive(Debug, Clone, Copy)]
pub(crate) struct ReadingHandshakeResponse {
    fd: BorrowedFd<'static>,
    reader: HttpLinesReader<HandshakeResponseParser>,
    last_activity_at: u64,
}

impl ReadingHandshakeResponse {
    pub(crate) fn new(fd: BorrowedFd<'static>, now: u64) -> Self {
        Self {
            fd,
            reader: HttpLinesReader::new(HandshakeResponseParser::new()),
            last_activity_at: now,
        }
    }

    pub(crate) fn wants(&self) -> Option<(BorrowedFd<'static>, Wants)> {
        Some((self.fd, Wants::Read))
    }

    pub(crate) fn disconnect_if_stuck(self, now: u64) -> ConnectionState {
        if now - self.last_activity_at > FREEZE_TIME_IN_SECS {
            error!("Stuck in ReadingHandshakeResponse, disconnecting...");
            self.disconnect(now).into()
        } else {
            self.into()
        }
    }

    pub(crate) fn disconnect(self, now: u64) -> ConnectionState {
        unsafe { rustix::io::close(self.fd.as_raw_fd()) };
        Disconnected::new(now).into()
    }

    pub(crate) fn read(mut self, now: u64) -> ConnectionState {
        match self.reader.read(&self.fd) {
            HttpLinesReaderResult::Done {
                buf,
                len,
                output: (),
            } => {
                trace!("Handshake response matches");

                if let Err(err) = enable_tcp_keep_alive(&self.fd) {
                    error!("{err:?}");
                    self.disconnect(now)
                } else {
                    let data = &buf[..len];
                    Connected::new(self.fd, data).into()
                }
            }
            HttpLinesReaderResult::Pending => {
                trace!("handshake response still pending: {:?}", self.reader);
                self.last_activity_at = now;
                self.into()
            }
            HttpLinesReaderResult::Err(err) => {
                error!("failed to read() handshake response: {err:?}");
                self.disconnect(now)
            }
        }
    }
}
