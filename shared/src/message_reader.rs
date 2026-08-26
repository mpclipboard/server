use crate::{Message, reader::Reader};

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct MessageReader {
    inner: Reader<{ Message::BYTESIZE }>,
}

impl MessageReader {
    pub const fn new() -> Self {
        Self {
            inner: Reader::new(),
        }
    }

    pub fn new_with_data(buf: &[u8]) -> (Self, Option<std::io::Result<Message>>) {
        let (inner, buf) = Reader::new_with_data(buf);
        let message = buf.map(|buf| Message::decode(&buf));

        (Self { inner }, message)
    }

    pub fn received(&mut self, data: &[u8]) -> (usize, Option<std::io::Result<Message>>) {
        let (consumed, buf) = self.inner.received(data);
        (consumed, buf.map(|buf| Message::decode(&buf)))
    }

    #[must_use]
    pub fn bytes_needed(&self) -> usize {
        self.inner.remaining()
    }
}

impl Default for MessageReader {
    fn default() -> Self {
        Self::new()
    }
}
