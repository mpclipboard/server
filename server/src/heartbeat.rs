use mpclipboard_shared::{
    ID, error,
    heartbeat::Beat,
    trace,
    writer::{Writer, WriterResult},
};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};

#[derive(Debug)]
pub struct Heartbeat {
    fd: OwnedFd,
    id: ID,
}

pub enum HeartbeatResult {
    Died,
    Ok,
}

impl Heartbeat {
    pub(crate) fn new(fd: OwnedFd, id: ID) -> Self {
        Self { fd, id }
    }

    pub(crate) fn tick(&mut self, _now: u64) -> HeartbeatResult {
        let mut writer = Writer::new(&Beat);
        match writer.write(&self.fd) {
            WriterResult::Done => {
                trace!("heartbeat sent to {self}");
                HeartbeatResult::Ok
            }
            WriterResult::StillPending => HeartbeatResult::Ok,
            WriterResult::Died(err) => {
                error!("failed to write() for {self}: {err:?}");
                HeartbeatResult::Died
            }
        }
    }
}

impl core::fmt::Display for Heartbeat {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Heartbeat({})", self.id)
    }
}

impl AsFd for Heartbeat {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for Heartbeat {
    fn as_raw_fd(&self) -> i32 {
        self.fd.as_raw_fd()
    }
}
