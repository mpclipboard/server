#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::Timerfd;
use anyhow::Result;
use std::{
    os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd},
    time::Duration,
};

pub struct EventLoop {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    epoll_fd: RawFd,
    #[cfg(any(target_os = "linux", target_os = "android"))]
    timer: Timerfd,

    fd1: FdState,
    fd2: FdState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wants {
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug)]
pub struct EventLoopResult {
    pub time: Option<u64>,
    pub fd1: Option<(bool, bool, bool)>,
    pub fd2: Option<(bool, bool, bool)>,
}

impl EventLoop {
    const TIMER_ID: u64 = 1;
    const ID1: u64 = 2;
    const ID2: u64 = 3;

    #[cfg(target_os = "macos")]
    pub fn new() -> Result<Self> {
        todo!("kqueue backend");
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub fn new() -> Result<Self> {
        use anyhow::Context;

        let epoll_fd = epoll::create(true).context("bug: failed to instantiate epoll")?;

        let mut this = Self {
            epoll_fd,
            timer: Timerfd::new(),
            fd1: FdState::new(),
            fd2: FdState::new(),
        };
        this.add_timer()?;

        Ok(this)
    }

    #[cfg(target_os = "macos")]
    pub fn sync(&mut self, _what: Option<(BorrowedFd<'static>, Wants)>) -> Result<()> {
        todo!("kqueue backend");
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub fn sync(
        &mut self,
        wants1: Option<(BorrowedFd<'static>, Wants)>,
        wants2: Option<(BorrowedFd<'static>, Wants)>,
    ) -> Result<()> {
        let diffs = [
            (Self::ID1, self.fd1.transition(wants1)),
            (Self::ID2, self.fd2.transition(wants2)),
        ];

        for (id, diff) in diffs {
            match diff {
                Diff::Add { fd, wants } => {
                    self.add(fd, id, wants)?;
                }
                Diff::Delete { fd } => {
                    self.delete(fd)?;
                }
                Diff::Modify { fd, wants } => {
                    self.modify(fd, id, wants)?;
                }
                Diff::Replace {
                    prevfd,
                    newfd,
                    wants,
                } => {
                    self.delete(prevfd)?;
                    self.add(newfd, id, wants)?;
                }
                Diff::Empty => {}
            }
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub fn wait(&mut self, _timeout: Option<Duration>) -> Result<EventLoopResult> {
        todo!("kqueue backend");
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub fn wait(&mut self, timeout: Option<Duration>) -> Result<EventLoopResult> {
        let mut events = [epoll::Event::new(epoll::Events::empty(), 0); 4];
        let len = epoll::wait(self.epoll_fd, Self::timeout_to_ms(timeout), &mut events)
            .map_err(|err| anyhow::anyhow!("bug: failed to read from epoll: {err:?}"))?;

        let mut out = EventLoopResult {
            time: None,
            fd1: None,
            fd2: None,
        };

        for event in events.iter().take(len) {
            match event.data {
                Self::TIMER_ID => {
                    let time = self.drain_timer();
                    out.time = Some(time);
                }

                Self::ID1 => {
                    let flags = epoll::Events::from_bits_retain(event.events);
                    out.fd1 = Some((
                        flags.contains(epoll::Events::EPOLLIN),
                        flags.contains(epoll::Events::EPOLLOUT),
                        flags.intersects(
                            epoll::Events::EPOLLERR
                                | epoll::Events::EPOLLHUP
                                | epoll::Events::EPOLLRDHUP,
                        ),
                    ));
                }

                Self::ID2 => {
                    let flags = epoll::Events::from_bits_retain(event.events);

                    out.fd2 = Some((
                        flags.contains(epoll::Events::EPOLLIN),
                        flags.contains(epoll::Events::EPOLLOUT),
                        flags.intersects(
                            epoll::Events::EPOLLERR
                                | epoll::Events::EPOLLHUP
                                | epoll::Events::EPOLLRDHUP,
                        ),
                    ));
                }

                _ => {
                    anyhow::bail!("bug: unknown event: {event:?}")
                }
            }
        }

        Ok(out)
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn add(&self, fd: BorrowedFd<'static>, id: u64, wants: Wants) -> Result<()> {
        epoll::ctl(
            self.epoll_fd,
            epoll::ControlOptions::EPOLL_CTL_ADD,
            fd.as_raw_fd(),
            Self::event(wants, id),
        )?;
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn delete(&self, fd: BorrowedFd<'static>) -> Result<()> {
        let _ = epoll::ctl(
            self.epoll_fd,
            epoll::ControlOptions::EPOLL_CTL_DEL,
            fd.as_raw_fd(),
            epoll::Event::new(epoll::Events::empty(), 0),
        );
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn modify(&self, fd: BorrowedFd<'static>, id: u64, wants: Wants) -> Result<()> {
        epoll::ctl(
            self.epoll_fd,
            epoll::ControlOptions::EPOLL_CTL_MOD,
            fd.as_raw_fd(),
            Self::event(wants, id),
        )?;
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn event(wants: Wants, id: u64) -> epoll::Event {
        let events = match wants {
            Wants::Read => epoll::Events::EPOLLIN,
            Wants::Write => epoll::Events::EPOLLOUT,
            Wants::ReadWrite => epoll::Events::EPOLLIN | epoll::Events::EPOLLOUT,
        } | epoll::Events::EPOLLRDHUP;
        epoll::Event::new(events, id)
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn timeout_to_ms(timeout: Option<Duration>) -> i32 {
        let Some(timeout) = timeout else {
            return -1;
        };

        i32::try_from(timeout.as_millis()).unwrap_or(i32::MAX)
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
impl AsFd for EventLoop {
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.epoll_fd) }
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
impl AsRawFd for EventLoop {
    fn as_raw_fd(&self) -> RawFd {
        self.epoll_fd
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
impl Drop for EventLoop {
    fn drop(&mut self) {
        let _ = epoll::close(self.epoll_fd);
    }
}

trait AddTimer {
    fn add_timer(&mut self) -> Result<()>;
    fn drain_timer(&mut self) -> u64;
}

#[cfg(target_os = "macos")]
impl AddTimer for EventLoop {
    fn add_timer(&mut self) -> Result<()> {
        todo!("kqueue backend");
    }

    #[cfg(target_os = "macos")]
    fn drain_timer(&mut self) -> u64 {
        todo!("kqueue backend");
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
impl AddTimer for EventLoop {
    fn add_timer(&mut self) -> Result<()> {
        epoll::ctl(
            self.epoll_fd,
            epoll::ControlOptions::EPOLL_CTL_ADD,
            self.timer.as_raw_fd(),
            epoll::Event::new(epoll::Events::EPOLLIN, Self::TIMER_ID),
        )
        .map_err(|err| anyhow::anyhow!("bug: failed to add timer to epoll: {err:?}"))?;
        Ok(())
    }

    fn drain_timer(&mut self) -> u64 {
        self.timer.read()
    }
}

#[derive(Debug, Clone, Copy)]
enum FdState {
    None,
    Some(BorrowedFd<'static>, Wants),
}

impl FdState {
    fn new() -> Self {
        Self::None
    }

    fn transition(&mut self, next: Option<(BorrowedFd<'static>, Wants)>) -> Diff {
        match (*self, next) {
            (Self::None, None) => Diff::Empty,
            (Self::None, Some((fd, wants))) => {
                *self = Self::Some(fd, wants);
                Diff::Add { fd, wants }
            }
            (Self::Some(prevfd, _), None) => {
                *self = Self::None;
                Diff::Delete { fd: prevfd }
            }
            (Self::Some(prevfd, prevwants), Some((fd, wants))) => {
                if fd.as_raw_fd() != prevfd.as_raw_fd() {
                    *self = Self::Some(fd, wants);
                    Diff::Replace {
                        prevfd: prevfd,
                        newfd: fd,
                        wants,
                    }
                } else if wants != prevwants {
                    *self = Self::Some(fd, wants);
                    Diff::Modify { fd, wants }
                } else {
                    Diff::Empty
                }
            }
        }
    }
}

enum Diff {
    Add {
        fd: BorrowedFd<'static>,
        wants: Wants,
    },
    Delete {
        fd: BorrowedFd<'static>,
    },
    Modify {
        fd: BorrowedFd<'static>,
        wants: Wants,
    },
    Replace {
        prevfd: BorrowedFd<'static>,
        newfd: BorrowedFd<'static>,
        wants: Wants,
    },
    Empty,
}
