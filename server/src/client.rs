use crate::{as_poll_fd::AsPollFd, revents::REvents};
use mpclipboard_shared::{
    ID, error,
    messaging::{
        message::Message,
        writer::{MessageWriter, MessageWriterResult},
    },
    reader::{Reader, ReaderResult},
    trace,
};
use rustix::event::{PollFd, PollFlags};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};

pub struct Client {
    fd: OwnedFd,
    id: ID,
    reader: Reader<{ Message::BYTESIZE }, Message>,
    writer: MessageWriter,
}

pub enum ClientResult {
    Died,
    Message((Message, Client)),
    StillPending(Client),
}

impl Client {
    pub(crate) fn new(fd: OwnedFd, id: ID) -> Self {
        Self {
            fd,
            id,
            reader: Reader::new(),
            writer: MessageWriter::new(),
        }
    }

    pub(crate) fn push(&mut self, message: &Message) {
        self.writer.push(message);
    }

    pub(crate) fn on_poll_event(mut self, revents: PollFlags) -> ClientResult {
        let revents = match REvents::new(revents) {
            Ok(revents) => revents,
            Err(err) => {
                error!("polling {self} returned an error: {err:?}");
                return ClientResult::Died;
            }
        };

        if revents.writable {
            trace!("{self} is writable");

            match self.writer.write(&self.fd) {
                MessageWriterResult::StillPending => {}
                MessageWriterResult::Died(err) => {
                    error!("failed to write() for {self}: {err:?}");
                    return ClientResult::Died;
                }
            }
        }

        if revents.readable {
            trace!("{self} is readable");

            match self.reader.read(&self.fd) {
                ReaderResult::StillPending => {}
                ReaderResult::Died(err) => {
                    error!("failed to read() for {self}: {err:?}");
                    return ClientResult::Died;
                }
                ReaderResult::Data(message) => {
                    return ClientResult::Message((message, self));
                }
            }
        }

        ClientResult::StillPending(self)
    }

    pub(crate) fn id(&self) -> ID {
        self.id
    }
}

impl core::fmt::Display for Client {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Client(fd={}, id={})", self.fd.as_raw_fd(), self.id)
    }
}

impl AsFd for Client {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for Client {
    fn as_raw_fd(&self) -> i32 {
        self.fd.as_raw_fd()
    }
}

impl AsPollFd for Client {
    fn as_poll_fd(&self) -> PollFd<'_> {
        let mut events = PollFlags::IN;
        if !self.writer.is_empty() {
            events |= PollFlags::OUT;
        }
        PollFd::new(&self.fd, events)
    }
}
