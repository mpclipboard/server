use crate::{as_poll_fd::AsPollFd, reaper::CanBeReaped};
use mpclipboard_shared::{
    Encode, ID, error,
    revents::REvents,
    trace,
    writer::{Writer, WriterResult},
};
use rustix::event::{PollFd, PollFlags};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};

pub struct PreSink<const N: usize, T>
where
    T: Encode<N>,
{
    fd: OwnedFd,
    id: ID,
    writer: Writer<N, T>,
    last_activity_at: u64,
}

pub enum PreSinkResult<const N: usize, T>
where
    T: Encode<N>,
{
    Died,
    StillPending(PreSink<N, T>),
    Done((ID, OwnedFd)),
}

impl<const N: usize, T> PreSink<N, T>
where
    T: Encode<N>,
{
    pub(crate) fn new(fd: OwnedFd, id: ID, now: u64, data: &T) -> Self {
        Self {
            fd,
            id,
            writer: Writer::new(data),
            last_activity_at: now,
        }
    }

    pub(crate) fn on_poll_event(mut self, revents: PollFlags, now: u64) -> PreSinkResult<N, T> {
        let revents = match REvents::new(revents) {
            Ok(revents) => revents,
            Err(err) => {
                error!("polling {self} returned an error: {err:?}");
                return PreSinkResult::Died;
            }
        };

        if revents.readable {
            unreachable!("{self} is readable but noone asked for it");
        }

        if revents.writable {
            trace!("{self} is writable");

            match self.writer.write(&self.fd) {
                WriterResult::Done => {
                    return PreSinkResult::Done((self.id, self.fd));
                }
                WriterResult::StillPending => {
                    self.last_activity_at = now;
                    return PreSinkResult::StillPending(self);
                }
                WriterResult::Died(err) => {
                    error!("{self} failed to write(): {err:?}");
                    return PreSinkResult::Died;
                }
            }
        }

        PreSinkResult::StillPending(self)
    }
}

impl<const N: usize, T> CanBeReaped for PreSink<N, T>
where
    T: Encode<N>,
{
    fn last_activity_at(&self) -> u64 {
        self.last_activity_at
    }
}

impl<const N: usize, T> core::fmt::Display for PreSink<N, T>
where
    T: Encode<N>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "PreSink(fd={}, last_activity_at={})",
            self.fd.as_raw_fd(),
            self.last_activity_at
        )
    }
}

impl<const N: usize, T> AsFd for PreSink<N, T>
where
    T: Encode<N>,
{
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl<const N: usize, T> AsRawFd for PreSink<N, T>
where
    T: Encode<N>,
{
    fn as_raw_fd(&self) -> i32 {
        self.fd.as_raw_fd()
    }
}

impl<const N: usize, T> AsPollFd for PreSink<N, T>
where
    T: Encode<N>,
{
    fn as_poll_fd(&self) -> PollFd<'_> {
        PollFd::new(&self.fd, PollFlags::OUT)
    }
}
