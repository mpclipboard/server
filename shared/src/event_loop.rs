#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::{Timerfd, Wants};
use core::time::Duration;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd};

pub struct EventLoop {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    epoll_fd: RawFd,
    #[cfg(any(target_os = "linux", target_os = "android"))]
    timer: Timerfd,

    fd: FdState,
}

#[derive(Debug)]
pub struct EventLoopResult {
    pub time: Option<u64>,
    pub fd: Option<(bool, bool, bool)>,
}

impl EventLoop {
    const TIMER_ID: u64 = 1;
    const FD_ID: u64 = 2;

    #[cfg(target_os = "macos")]
    pub fn new() -> Result<Self, EventLoopError> {
        todo!("kqueue backend");
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub fn new() -> std::io::Result<Self> {
        let epoll_fd = epoll::create(true)?;

        let mut this = Self {
            epoll_fd,
            timer: Timerfd::new()?,
            fd: FdState::new(),
        };
        this.add_timer()?;

        Ok(this)
    }

    #[cfg(target_os = "macos")]
    pub fn sync(
        &mut self,
        _what: Option<(BorrowedFd<'static>, Wants)>,
    ) -> Result<(), EventLoopError> {
        todo!("kqueue backend");
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub fn sync(&mut self, wants: Option<(BorrowedFd<'static>, Wants)>) -> std::io::Result<()> {
        match self.fd.transition(wants) {
            Diff::Add { fd, wants } => {
                self.add(fd, Self::FD_ID, wants)?;
            }
            Diff::Delete { fd } => {
                self.delete(fd);
            }
            Diff::Modify { fd, wants } => {
                self.modify(fd, Self::FD_ID, wants)?;
            }
            Diff::Replace {
                prevfd,
                newfd,
                wants,
            } => {
                self.delete(prevfd);
                self.add(newfd, Self::FD_ID, wants)?;
            }
            Diff::Empty => {}
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub fn wait(&mut self, _timeout: Option<Duration>) -> Result<EventLoopResult, EventLoopError> {
        todo!("kqueue backend");
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub fn wait(&mut self, timeout: Option<Duration>) -> std::io::Result<EventLoopResult> {
        let mut events = [epoll::Event::new(epoll::Events::empty(), 0); 4];
        let len = epoll::wait(self.epoll_fd, Self::timeout_to_ms(timeout), &mut events)?;

        let mut out = EventLoopResult {
            time: None,
            fd: None,
        };

        for event in events.iter().take(len) {
            match event.data {
                Self::TIMER_ID => {
                    let time = self.drain_timer()?;
                    out.time = Some(time);
                }

                Self::FD_ID => {
                    let flags = epoll::Events::from_bits_retain(event.events);
                    out.fd = Some((
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
                    let id = event.data;
                    return Err(std::io::Error::other(format!("unknown event {id}")));
                }
            }
        }

        Ok(out)
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn add(&self, fd: BorrowedFd<'static>, id: u64, wants: Wants) -> std::io::Result<()> {
        epoll::ctl(
            self.epoll_fd,
            epoll::ControlOptions::EPOLL_CTL_ADD,
            fd.as_raw_fd(),
            Self::event(wants, id),
        )?;
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn delete(&self, fd: BorrowedFd<'static>) {
        let _ = epoll::ctl(
            self.epoll_fd,
            epoll::ControlOptions::EPOLL_CTL_DEL,
            fd.as_raw_fd(),
            epoll::Event::new(epoll::Events::empty(), 0),
        );
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn modify(&self, fd: BorrowedFd<'static>, id: u64, wants: Wants) -> std::io::Result<()> {
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
    fn add_timer(&mut self) -> std::io::Result<()>;
    fn drain_timer(&mut self) -> std::io::Result<u64>;
}

#[cfg(target_os = "macos")]
impl AddTimer for EventLoop {
    fn add_timer(&mut self) -> Result<(), EventLoopError> {
        todo!("kqueue backend");
    }

    #[cfg(target_os = "macos")]
    fn drain_timer(&mut self) -> Result<u64, EventLoopError> {
        todo!("kqueue backend");
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
impl AddTimer for EventLoop {
    fn add_timer(&mut self) -> std::io::Result<()> {
        epoll::ctl(
            self.epoll_fd,
            epoll::ControlOptions::EPOLL_CTL_ADD,
            self.timer.as_raw_fd(),
            epoll::Event::new(epoll::Events::EPOLLIN, Self::TIMER_ID),
        )?;
        Ok(())
    }

    fn drain_timer(&mut self) -> std::io::Result<u64> {
        self.timer.read()
    }
}

#[must_use]
#[derive(Debug, Clone, Copy)]
enum FdState {
    None,
    Some(BorrowedFd<'static>, Wants),
}

impl FdState {
    const fn new() -> Self {
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
                        prevfd,
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

#[must_use]
#[derive(Debug)]
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
