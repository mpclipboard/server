use rustix::event::PollFlags;

#[derive(Debug, Clone, Copy)]
pub struct REvents {
    pub readable: bool,
    pub writable: bool,
}

impl REvents {
    pub fn new(revents: PollFlags) -> std::io::Result<Self> {
        if revents.intersects(PollFlags::HUP | PollFlags::ERR | PollFlags::NVAL) {
            return Err(std::io::Error::other(format!("got revents: {revents:?}")));
        }
        let readable = revents.contains(PollFlags::IN);
        let writable = revents.contains(PollFlags::OUT);
        Ok(Self { readable, writable })
    }
}
