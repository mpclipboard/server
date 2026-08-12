use crate::{as_poll_fd::AsPollFd, reaper::CanBeReaped};
use mpclipboard_shared::{
    byte_stream::PlainByteStream,
    error,
    http_lines_reader::{HttpLinesParser, HttpLinesReader, HttpLinesReaderResult},
    revents::REvents,
    trace,
};
use rustix::event::{PollFd, PollFlags};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};

pub struct PreSource<P>
where
    P: HttpLinesParser,
{
    fd: OwnedFd,
    reader: HttpLinesReader<P>,
    last_activity_at: u64,
}

pub enum PreSourceResult<P>
where
    P: HttpLinesParser,
{
    Died,
    StillPending(PreSource<P>),
    Done((P::Output, OwnedFd)),
}

impl<P> PreSource<P>
where
    P: HttpLinesParser,
{
    pub(crate) fn new(fd: OwnedFd, now: u64) -> Self {
        Self {
            fd,
            reader: HttpLinesReader::new(P::new()),
            last_activity_at: now,
        }
    }

    pub(crate) fn on_poll_event(mut self, revents: PollFlags, now: u64) -> PreSourceResult<P> {
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

impl<P> CanBeReaped for PreSource<P>
where
    P: HttpLinesParser,
{
    fn last_activity_at(&self) -> u64 {
        self.last_activity_at
    }
}

impl<P> core::fmt::Display for PreSource<P>
where
    P: HttpLinesParser,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "PreSource(fd={}, last_activity_at={})",
            self.fd.as_raw_fd(),
            self.last_activity_at
        )
    }
}

impl<P> AsFd for PreSource<P>
where
    P: HttpLinesParser,
{
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl<P> AsRawFd for PreSource<P>
where
    P: HttpLinesParser,
{
    fn as_raw_fd(&self) -> i32 {
        self.fd.as_raw_fd()
    }
}

impl<P> AsPollFd for PreSource<P>
where
    P: HttpLinesParser,
{
    fn as_poll_fd(&self) -> PollFd<'_> {
        PollFd::new(&self.fd, PollFlags::IN)
    }
}
