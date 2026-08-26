use crate::connection::{
    ConnectionState, FREEZE_TIME_IN_SECS, connected::Connected, disconnected::Disconnected,
    maybe_tls_stream::MaybeTlsStream,
};
use mpclipboard_shared::{
    HandshakeResponseReader, Message, Wants, enable_tcp_keep_alive, error, trace, warn,
};
use std::os::fd::{AsRawFd, BorrowedFd};

#[derive(Debug, Clone, Copy)]
pub struct ReadingHandshakeResponse {
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

    pub(crate) fn wants(&self, stream: &MaybeTlsStream) -> (BorrowedFd<'static>, Wants) {
        (self.fd, stream.wants(Wants::Read))
    }

    pub(crate) const fn fd(&self) -> BorrowedFd<'static> {
        self.fd
    }

    pub(crate) fn disconnect_if_stuck(self, now: u64) -> ConnectionState {
        let diff = now
            .checked_sub(self.last_activity_at)
            .unwrap_or_else(|| unreachable!("time goes backwards"));
        if diff > FREEZE_TIME_IN_SECS {
            error!("Stuck in ReadingHandshakeResponse, disconnecting...");
            self.disconnect(now)
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
        let mut buf = [0; Message::BYTESIZE];
        let len = match stream.read_bytes(&self.fd, &mut buf) {
            Ok(Some(len)) => len.get(),
            Ok(None) => {
                trace!("handshake response still pending: {:?}", self.reader);
                self.last_activity_at = now;
                return (self.into(), None);
            }
            Err(err) => {
                error!("failed to read() handshake response: {err:?}");
                return (self.disconnect(now), None);
            }
        };

        let data = buf
            .get(..len)
            .unwrap_or_else(|| unreachable!("stream returned an oversized read"));
        match self.reader.received(data) {
            Ok((consumed, Some(()))) => {
                trace!("Handshake response matches");

                if let Err(err) = enable_tcp_keep_alive(&self.fd) {
                    error!("{err:?}");
                    (self.disconnect(now), None)
                } else {
                    let leftover = buf
                        .get(consumed..len)
                        .unwrap_or_else(|| unreachable!("leftover buffer is malformed"));
                    warn!("Handshake leftover: {leftover:?}");
                    let (connected, message) = Connected::new(self.fd, leftover);
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
            Ok((_, None)) => {
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
