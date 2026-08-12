use crate::connection::{
    ConnectionState, FREEZE_TIME_IN_SECS, connected::Connected, disconnected::Disconnected,
    stream::Stream,
};
use mpclipboard_shared::{
    error,
    event_loop::Wants,
    handshake_response::HandshakeResponseParser,
    http_lines_reader::{HttpLinesParser, HttpLinesReader, HttpLinesReaderResult},
    message::Message,
    reader::ReaderResult,
    tcp_keep_alive::enable_tcp_keep_alive,
    trace, warn,
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

    pub(crate) fn wants(&self, stream: &Stream) -> Option<(BorrowedFd<'static>, Wants)> {
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
        stream: &mut Stream,
    ) -> (ConnectionState, Option<Message>) {
        match self.reader.read_from(stream, &self.fd) {
            HttpLinesReaderResult::Done {
                buf,
                len,
                output: (),
            } => {
                trace!("Handshake response matches");

                if let Err(err) = enable_tcp_keep_alive(&self.fd) {
                    error!("{err:?}");
                    (self.disconnect(now), None)
                } else {
                    let data = &buf[..len];
                    warn!("Handshake leftover: {data:?}");
                    let (connected, res) = Connected::new(self.fd, data);
                    match res {
                        Some(ReaderResult::Data(message)) => (connected.into(), Some(message)),
                        Some(ReaderResult::Died(err)) => {
                            error!("failed to read handshake leftover: {err:?}");
                            (connected.disconnect(now), None)
                        }
                        Some(ReaderResult::StillPending) => {
                            unreachable!("Reader::new_with_data never returns StillPending")
                        }
                        None => connected.read(now, stream),
                    }
                }
            }
            HttpLinesReaderResult::Pending => {
                trace!("handshake response still pending: {:?}", self.reader);
                self.last_activity_at = now;
                (self.into(), None)
            }
            HttpLinesReaderResult::Err(err) => {
                error!("failed to read() handshake response: {err:?}");
                (self.disconnect(now), None)
            }
        }
    }
}
