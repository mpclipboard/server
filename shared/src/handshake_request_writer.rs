use crate::{HandshakeRequest, byte_stream::ByteStream, writer::Writer};
use std::os::fd::AsFd;

#[derive(Debug, Clone, Copy)]
pub struct HandshakeRequestWriter {
    inner: Writer<{ HandshakeRequest::BYTESIZE }>,
}

impl HandshakeRequestWriter {
    pub fn new(request: &HandshakeRequest) -> Self {
        Self {
            inner: Writer::new(request.encode()),
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
