use crate::trace;
use core::num::NonZeroUsize;

#[derive(Clone, Copy)]
pub struct Readbuf<const N: usize> {
    buf: [u8; N],
    pos: usize,
}

impl<const N: usize> Readbuf<N> {
    pub(crate) const fn new() -> Self {
        Self {
            buf: [0; _],
            pos: 0,
        }
    }

    pub(crate) fn new_with_data(data: &[u8]) -> Self {
        assert!(data.len() <= N);
        trace!("starting with {} leftover bytes", data.len());

        let mut buf = [0; N];
        buf[..data.len()].copy_from_slice(data);

        Self {
            buf,
            pos: data.len(),
        }
    }

    pub(crate) fn remainder(&mut self) -> &mut [u8] {
        &mut self.buf[self.pos..]
    }

    pub(crate) fn received(&mut self, n: NonZeroUsize) -> Option<[u8; N]> {
        self.pos = self
            .pos
            .checked_add(n.get())
            .unwrap_or_else(|| unreachable!("overflow: n is too large"));

        assert!(self.pos <= N);

        if self.pos == N {
            let buf = self.buf;

            self.pos = 0;
            self.buf.fill(0);

            Some(buf)
        } else {
            None
        }
    }
}

impl<const N: usize> core::fmt::Debug for Readbuf<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Readbuf")
            .field("buf", &core::str::from_utf8(&self.buf))
            .field("pos", &self.pos)
            .finish()
    }
}
