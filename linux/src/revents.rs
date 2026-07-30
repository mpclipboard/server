use anyhow::{Result, bail};
use rustix::event::PollFlags;

pub struct REvents {
    pub(crate) readable: bool,
    pub(crate) writable: bool,
}

impl REvents {
    pub(crate) fn new(name: &str, revents: PollFlags) -> Result<Self> {
        if revents.contains(PollFlags::NVAL) {
            bail!("{name} poll failed: invalid fd");
        } else if revents.contains(PollFlags::ERR) {
            bail!("{name} poll failed: fd error");
        } else if revents.intersects(PollFlags::HUP | PollFlags::RDHUP) {
            bail!("{name} poll failed: hangup");
        }

        let readable = revents.contains(PollFlags::IN);
        let writable = revents.contains(PollFlags::OUT);

        Ok(Self { readable, writable })
    }
}
