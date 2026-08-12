use crate::{
    byte_stream::{ByteStream, WriteResult},
    message::Message,
    writebuf::Writebuf,
};
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

#[derive(Debug)]
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
        let item = Writebuf::new(data.encode());

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

    pub fn write_to(
        &mut self,
        stream: &mut impl ByteStream,
        fd: &impl AsFd,
    ) -> MessageWriterResult {
        while let Some(buf) = self.buf_to_write() {
            match stream.write_bytes(fd, buf) {
                WriteResult::Data(len) => self.written(len),
                WriteResult::Eof => return MessageWriterResult::Died(MessageWriterError::EOF),
                WriteResult::WouldBlock => return MessageWriterResult::StillPending,
                WriteResult::Err(err) => {
                    return MessageWriterResult::Died(MessageWriterError::Transport(err));
                }
            }
        }

        MessageWriterResult::StillPending
    }

    pub fn is_empty(&self) -> bool {
        self.buf_to_write().is_none()
    }
}

#[derive(Debug)]
pub enum MessageWriterError {
    EOF,
    Transport(anyhow::Error),
}
