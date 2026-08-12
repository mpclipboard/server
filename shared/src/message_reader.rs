use crate::{ByteStream, Message, reader::Reader};
use std::os::fd::AsFd;

#[derive(Debug, Clone, Copy)]
pub struct MessageReader {
    inner: Reader<{ Message::BYTESIZE }>,
}

impl MessageReader {
    pub fn new() -> Self {
        Self {
            inner: Reader::new(),
        }
    }

    pub fn new_with_data(buf: &[u8]) -> (Self, Option<std::io::Result<Message>>) {
        let (inner, buf) = Reader::new_with_data(buf);
        let message = buf.map(|buf| Message::decode(&buf));

        (Self { inner }, message)
    }

    pub fn read_from(
        &mut self,
        stream: &mut impl ByteStream,
        fd: &impl AsFd,
    ) -> std::io::Result<Option<Message>> {
        let Some(buf) = self.inner.read_from(stream, fd)? else {
            return Ok(None);
        };
        let message = Message::decode(&buf)?;
        Ok(Some(message))
    }
}
