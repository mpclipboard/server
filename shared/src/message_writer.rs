use crate::{message::Message, writebuf::Writebuf};
use core::num::NonZeroUsize;

#[must_use]
#[expect(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy)]
pub enum MessageWriter {
    Empty,

    Some {
        current: Writebuf<{ Message::BYTESIZE }>,
        next: Option<Writebuf<{ Message::BYTESIZE }>>,
    },
}

impl MessageWriter {
    pub const fn new() -> Self {
        Self::Empty
    }

    #[must_use]
    pub fn remainder(&self) -> Option<&[u8]> {
        match self {
            Self::Empty => None,
            Self::Some { current, .. } => Some(current.remainder()),
        }
    }

    pub fn written(&mut self, n: NonZeroUsize) {
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

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.remainder().is_none()
    }
}

impl Default for MessageWriter {
    fn default() -> Self {
        Self::new()
    }
}
