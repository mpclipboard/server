use crate::{as_poll_fd::AsPollFd, reaper::CanBeReaped};
use mpclipboard_shared::{
    byte_stream::PlainByteStream,
    error,
    handshake_request::{HandshakeRequest, HandshakeRequestParser, HandshakeRequestReader},
    http_lines_reader::{HttpLinesParser, HttpLinesReaderResult},
    revents::REvents,
    trace,
};
use rustix::event::{PollFd, PollFlags};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};

pub struct PreSource {
    fd: OwnedFd,
    reader: HandshakeRequestReader,
    last_activity_at: u64,
}

pub enum PreSourceResult {
    Died,
    StillPending(PreSource),
    Done((HandshakeRequest, OwnedFd)),
}

impl PreSource {
    pub(crate) fn new(fd: OwnedFd, now: u64) -> Self {
        Self {
            fd,
            reader: HandshakeRequestReader::new(HandshakeRequestParser::new()),
            last_activity_at: now,
        }
    }

    pub(crate) fn on_poll_event(mut self, revents: PollFlags, now: u64) -> PreSourceResult {
        let revents = match REvents::new(revents) {
            Ok(revents) => revents,
            Err(err) => {
                error!("polling {self} returned an error: {err:?}");
                return PreSourceResult::Died;
            }
        };

        if revents.writable {
            unreachable!("{self} is writable but noone asked for it");
        }

        if revents.readable {
            trace!("{self} is readable");

            match self.reader.read_from(&mut PlainByteStream, &self.fd) {
                HttpLinesReaderResult::Done {
                    buf,
                    len,
                    output: req,
                } => {
                    let buf = &buf[..len];
                    assert!(buf.is_empty());
                    return PreSourceResult::Done((req, self.fd));
                }
                HttpLinesReaderResult::Pending => {
                    self.last_activity_at = now;
                    return PreSourceResult::StillPending(self);
                }
                HttpLinesReaderResult::Err(err) => {
                    error!("{self} failed to read(): {err:?}");
                    return PreSourceResult::Died;
                }
            }
        }

        PreSourceResult::StillPending(self)
    }
}

impl CanBeReaped for PreSource {
    fn last_activity_at(&self) -> u64 {
        self.last_activity_at
    }
}

impl core::fmt::Display for PreSource {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "PreSource(fd={}, last_activity_at={})",
            self.fd.as_raw_fd(),
            self.last_activity_at
        )
    }
}

impl AsFd for PreSource {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for PreSource {
    fn as_raw_fd(&self) -> i32 {
        self.fd.as_raw_fd()
    }
}

impl AsPollFd for PreSource {
    fn as_poll_fd(&self) -> PollFd<'_> {
        PollFd::new(&self.fd, PollFlags::IN)
    }
}
