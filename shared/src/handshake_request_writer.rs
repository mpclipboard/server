use crate::{HandshakeRequest, writer::Writer};
use core::num::NonZeroUsize;

#[must_use]
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

    #[must_use]
    pub fn remainder(&self) -> &[u8] {
        self.inner.remainder()
    }

    pub fn written(&mut self, len: NonZeroUsize) -> bool {
        self.inner.written(len)
    }
}
