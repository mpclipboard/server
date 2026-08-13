use crate::{byte_stream::ByteStream, handshake_response::HandshakeResponse, writer::Writer};
use std::os::fd::AsFd;

#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct HandshakeResponseWriter {
    inner: Writer<{ HandshakeResponse::BYTESIZE }>,
}

impl HandshakeResponseWriter {
    pub const fn new() -> Self {
        let mut buf = [0; HandshakeResponse::BYTESIZE];
        buf.copy_from_slice(HandshakeResponse::BYTES);

        Self {
            inner: Writer::new(buf),
        }
    }

    pub fn write_to(
        &mut self,
        stream: &mut impl ByteStream,
        fd: &impl AsFd,
    ) -> std::io::Result<bool> {
        self.inner.write_to(stream, fd)
    }
}

impl Default for HandshakeResponseWriter {
    fn default() -> Self {
        Self::new()
    }
}
