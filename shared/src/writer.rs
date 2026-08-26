use crate::writebuf::Writebuf;
use core::num::NonZeroUsize;

#[derive(Debug, Clone, Copy)]
pub struct Writer<const N: usize> {
    writebuf: Writebuf<N>,
}

impl<const N: usize> Writer<N> {
    pub(crate) const fn new(buf: [u8; N]) -> Self {
        Self {
            writebuf: Writebuf::new(buf),
        }
    }

    pub(crate) fn remainder(&self) -> &[u8] {
        self.writebuf.remainder()
    }

    pub(crate) fn written(&mut self, len: NonZeroUsize) -> bool {
        self.writebuf.written(len)
    }
}
