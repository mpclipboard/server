use mpclipboard_shared::Timerfd;
use rustix::event::{PollFd, PollFlags};
use std::os::fd::AsFd;

pub trait AsPollFd {
    fn as_poll_fd(&self) -> PollFd<'_>;
}

impl AsPollFd for Timerfd {
    fn as_poll_fd(&self) -> PollFd<'_> {
        PollFd::from_borrowed_fd(self.as_fd(), PollFlags::IN)
    }
}
