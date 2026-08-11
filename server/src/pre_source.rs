use crate::{as_poll_fd::AsPollFd, reaper::CanBeReaped, revents::REvents};
use mpclipboard_shared::{
    Decode, error,
    reader::{Reader, ReaderResult},
    trace,
};
use rustix::event::{PollFd, PollFlags};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};

pub struct PreSource<const N: usize, T>
where
    T: Decode<N>,
{
    fd: OwnedFd,
    reader: Reader<N, T>,
    last_activity_at: u64,
}

pub enum PreSourceResult<const N: usize, T>
where
    T: Decode<N>,
{
    Died,
    StillPending(PreSource<N, T>),
    Done((T, OwnedFd)),
}

impl<const N: usize, T> PreSource<N, T>
where
    T: Decode<N>,
{
    pub(crate) fn new(fd: OwnedFd, now: u64) -> Self {
        Self {
            fd,
            reader: Reader::new(),
            last_activity_at: now,
        }
    }

    pub(crate) fn on_poll_event(mut self, revents: PollFlags, now: u64) -> PreSourceResult<N, T> {
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
            match self.reader.read(&self.fd) {
                ReaderResult::Data(req) => {
                    return PreSourceResult::Done((req, self.fd));
                }
                ReaderResult::StillPending => {
                    self.last_activity_at = now;
                    return PreSourceResult::StillPending(self);
                }
                ReaderResult::Died(err) => {
                    error!("{self} failed to read(): {err:?}");
                    return PreSourceResult::Died;
                }
            }
        }

        PreSourceResult::StillPending(self)
    }
}

impl<const N: usize, T> CanBeReaped for PreSource<N, T>
where
    T: Decode<N>,
{
    fn last_activity_at(&self) -> u64 {
        self.last_activity_at
    }
}

impl<const N: usize, T> core::fmt::Display for PreSource<N, T>
where
    T: Decode<N>,
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

impl<const N: usize, T> AsFd for PreSource<N, T>
where
    T: Decode<N>,
{
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl<const N: usize, T> AsRawFd for PreSource<N, T>
where
    T: Decode<N>,
{
    fn as_raw_fd(&self) -> i32 {
        self.fd.as_raw_fd()
    }
}

impl<const N: usize, T> AsPollFd for PreSource<N, T>
where
    T: Decode<N>,
{
    fn as_poll_fd(&self) -> PollFd<'_> {
        PollFd::new(&self.fd, PollFlags::IN)
    }
}
