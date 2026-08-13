use super::{Diff, EventLoopResult, FdState};
use crate::{Timerfd, Wants};
use core::time::Duration;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd};

pub struct EventLoop {
    epoll_fd: RawFd,
    timer: Timerfd,
    fd: FdState,
}

impl EventLoop {
    const TIMER_ID: u64 = 1;
    const FD_ID: u64 = 2;

    pub fn new() -> std::io::Result<Self> {
        let epoll_fd = epoll::create(true)?;

        let this = Self {
            epoll_fd,
            timer: Timerfd::new()?,
            fd: FdState::new(),
        };
        this.add_timer()?;

        Ok(this)
    }

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

    fn add(&self, fd: BorrowedFd<'static>, id: u64, wants: Wants) -> std::io::Result<()> {
        epoll::ctl(
            self.epoll_fd,
            epoll::ControlOptions::EPOLL_CTL_ADD,
            fd.as_raw_fd(),
            Self::event(wants, id),
        )?;
        Ok(())
    }

    fn delete(&self, fd: BorrowedFd<'static>) {
        let _ = epoll::ctl(
            self.epoll_fd,
            epoll::ControlOptions::EPOLL_CTL_DEL,
            fd.as_raw_fd(),
            epoll::Event::new(epoll::Events::empty(), 0),
        );
    }

    fn modify(&self, fd: BorrowedFd<'static>, id: u64, wants: Wants) -> std::io::Result<()> {
        epoll::ctl(
            self.epoll_fd,
            epoll::ControlOptions::EPOLL_CTL_MOD,
            fd.as_raw_fd(),
            Self::event(wants, id),
        )?;
        Ok(())
    }

    fn event(wants: Wants, id: u64) -> epoll::Event {
        let events = match wants {
            Wants::Read => epoll::Events::EPOLLIN,
            Wants::Write => epoll::Events::EPOLLOUT,
            Wants::ReadWrite => epoll::Events::EPOLLIN | epoll::Events::EPOLLOUT,
        } | epoll::Events::EPOLLRDHUP;
        epoll::Event::new(events, id)
    }

    fn timeout_to_ms(timeout: Option<Duration>) -> i32 {
        let Some(timeout) = timeout else {
            return -1;
        };

        i32::try_from(timeout.as_millis()).unwrap_or(i32::MAX)
    }

    fn add_timer(&self) -> std::io::Result<()> {
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

impl AsFd for EventLoop {
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.epoll_fd) }
    }
}

impl AsRawFd for EventLoop {
    fn as_raw_fd(&self) -> RawFd {
        self.epoll_fd
    }
}

impl Drop for EventLoop {
    fn drop(&mut self) {
        let _ = epoll::close(self.epoll_fd);
    }
}
