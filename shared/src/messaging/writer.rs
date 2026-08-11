use crate::{Encode, messaging::message::Message, writebuf::Writebuf};
use rustix::io::Errno;
use std::{num::NonZeroUsize, os::fd::AsFd};

#[expect(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy)]
pub enum MessageWriter {
    Empty,

    Some {
        current: Writebuf<{ Message::BYTESIZE }>,
        next: Option<Writebuf<{ Message::BYTESIZE }>>,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum MessageWriterResult {
    StillPending,
    Died(MessageWriterError),
}

impl MessageWriter {
    pub fn new() -> Self {
        Self::Empty
    }

    #[must_use]
    fn buf_to_write(&self) -> Option<&[u8]> {
        match self {
            Self::Empty => None,
            Self::Some { current, .. } => Some(current.remainder()),
        }
    }

    fn written(&mut self, n: NonZeroUsize) {
        match self {
            Self::Empty => unreachable!("empty buffer never wants to write"),

            Self::Some { current, next } => {
                if current.written(n) {
                    if let Some(next) = core::mem::take(next) {
                        *current = next;
                    } else {
                        *self = Self::new();
                    }
                }
            }
        }
    }

    pub fn push(&mut self, data: &Message) {
        let mut buf = [0; Message::BYTESIZE];
        data.encode(&mut buf);
        let item = Writebuf::new(buf);

        match self {
            Self::Empty => {
                *self = Self::Some {
                    current: item,
                    next: None,
                }
            }
            Self::Some { next, .. } => *next = Some(item),
        }
    }

    pub fn write(&mut self, fd: &impl AsFd) -> MessageWriterResult {
        while let Some(buf) = self.buf_to_write() {
            let res = rustix::io::write(fd, buf).map(NonZeroUsize::new);

            match res {
                Ok(Some(len)) => self.written(len),
                Ok(None) => return MessageWriterResult::Died(MessageWriterError::EOF),
                Err(Errno::AGAIN) => return MessageWriterResult::StillPending,
                Err(errno) => return MessageWriterResult::Died(MessageWriterError::Errno(errno)),
            }
        }

        MessageWriterResult::StillPending
    }

    pub fn is_empty(&self) -> bool {
        self.buf_to_write().is_none()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum MessageWriterError {
    EOF,
    Errno(Errno),
}
