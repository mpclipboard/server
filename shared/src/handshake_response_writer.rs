use crate::{handshake_response::HandshakeResponse, writer::Writer};
use core::num::NonZeroUsize;

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

    #[must_use]
    pub fn remainder(&self) -> &[u8] {
        self.inner.remainder()
    }

    pub fn written(&mut self, len: NonZeroUsize) -> bool {
        self.inner.written(len)
    }
}

impl Default for HandshakeResponseWriter {
    fn default() -> Self {
        Self::new()
    }
}
