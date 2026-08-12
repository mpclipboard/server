use crate::connection::{
    ConnectionState, FREEZE_TIME_IN_SECS, connected::Connected, disconnected::Disconnected,
    maybe_tls_stream::MaybeTlsStream,
};
use mpclipboard_shared::{
    HandshakeResponseReader, Message, Wants, enable_tcp_keep_alive, error, trace, warn,
};
use std::os::fd::{AsRawFd, BorrowedFd};

#[derive(Debug, Clone, Copy)]
pub(crate) struct ReadingHandshakeResponse {
    fd: BorrowedFd<'static>,
    reader: HandshakeResponseReader,
    last_activity_at: u64,
}

impl ReadingHandshakeResponse {
    pub(crate) fn new(fd: BorrowedFd<'static>, now: u64) -> Self {
        Self {
            fd,
            reader: HandshakeResponseReader::new(),
            last_activity_at: now,
        }
    }

    pub(crate) fn wants(&self, stream: &MaybeTlsStream) -> Option<(BorrowedFd<'static>, Wants)> {
        Some((self.fd, stream.wants(Wants::Read)))
    }

    pub(crate) fn fd(&self) -> BorrowedFd<'static> {
        self.fd
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

    pub(crate) fn read(
        mut self,
        now: u64,
        stream: &mut MaybeTlsStream,
    ) -> (ConnectionState, Option<Message>) {
        match self.reader.read_from(stream, &self.fd) {
            Ok(Some(((), buf, len))) => {
                trace!("Handshake response matches");

                if let Err(err) = enable_tcp_keep_alive(&self.fd) {
                    error!("{err:?}");
                    (self.disconnect(now), None)
                } else {
                    let data = &buf[..len];
                    warn!("Handshake leftover: {data:?}");
                    let (connected, message) = Connected::new(self.fd, data);
                    match message {
                        Some(Ok(message)) => (connected.into(), Some(message)),
                        Some(Err(err)) => {
                            error!("failed to decode handshake leftover: {err:?}");
                            (connected.disconnect(now), None)
                        }
                        None => connected.read(now, stream),
                    }
                }
            }
            Ok(None) => {
                trace!("handshake response still pending: {:?}", self.reader);
                self.last_activity_at = now;
                (self.into(), None)
            }
            Err(err) => {
                error!("failed to read() handshake response: {err:?}");
                (self.disconnect(now), None)
            }
        }
    }
}
