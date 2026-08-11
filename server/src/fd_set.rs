use crate::{as_poll_fd::AsPollFd, reaper::CanBeReaped};
use rustix::event::PollFd;
use std::{
    collections::HashMap,
    os::fd::{AsFd, AsRawFd},
};

pub struct FdSet<const MAX: usize, T: AsFd> {
    map: HashMap<i32, T>,
}

impl<const MAX: usize, T: AsFd> Default for FdSet<MAX, T> {
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

impl<const MAX: usize, T: AsFd> FdSet<MAX, T> {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn insert(&mut self, value: T) {
        if self.map.len() < MAX {
            self.map.insert(value.as_fd().as_raw_fd(), value);
        }
    }

    pub(crate) fn remove(&mut self, fd: i32) -> Option<T> {
        self.map.remove(&fd)
    }

    pub(crate) fn fds(&self) -> impl Iterator<Item = &T> {
        self.map.values()
    }

    pub(crate) fn fds_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.map.values_mut()
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (&i32, &mut T)> {
        self.map.iter_mut()
    }
}

impl<const MAX: usize, T: AsFd + CanBeReaped> FdSet<MAX, T> {
    pub(crate) fn reap(&mut self, now: u64) {
        let mut fds_to_drop = vec![];
        for (fd, stranger_source) in &self.map {
            if stranger_source.must_be_reaped(now) {
                fds_to_drop.push(*fd);
            }
        }
        for fd in fds_to_drop {
            self.map.remove(&fd);
        }
    }
}

impl<const MAX: usize, T: AsFd + AsPollFd> FdSet<MAX, T> {
    pub(crate) fn as_poll_fds(&self) -> impl Iterator<Item = PollFd<'_>> {
        self.fds().map(|fd| fd.as_poll_fd())
    }
}
