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

    pub(crate) fn received(&mut self, data: &[u8]) -> (usize, Option<[u8; N]>) {
        let remaining = self.remaining();
        let len = data.len().min(remaining);
        let end = self
            .pos
            .checked_add(len)
            .unwrap_or_else(|| unreachable!("read position overflow"));
        self.buf
            .get_mut(self.pos..end)
            .unwrap_or_else(|| unreachable!("read range exceeds buffer"))
            .copy_from_slice(
                data.get(..len)
                    .unwrap_or_else(|| unreachable!("read range exceeds input")),
            );
        let n = NonZeroUsize::new(len);
        let Some(n) = n else { return (0, None) };
        self.pos = self
            .pos
            .checked_add(n.get())
            .unwrap_or_else(|| unreachable!("overflow: n is too large"));

        assert!(self.pos <= N);

        if self.pos == N {
            let buf = self.buf;

            self.pos = 0;
            self.buf.fill(0);

            (len, Some(buf))
        } else {
            (len, None)
        }
    }

    pub(crate) fn remaining(&self) -> usize {
        N.checked_sub(self.pos)
            .unwrap_or_else(|| unreachable!("malformed Readbuf pos"))
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
