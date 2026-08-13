use core::num::NonZeroUsize;

#[must_use]
#[derive(Clone, Copy)]
pub struct NonEmptyInlineString<const LEN: usize> {
    len: NonZeroUsize,
    bytes: [u8; LEN],
}

impl<const LEN: usize> NonEmptyInlineString<LEN> {
    #[must_use]
    pub fn truncate(s: &str) -> Option<Self> {
        let mut bytes_written: usize = 0;
        let mut bytes = [0; LEN];

        for c in s.chars() {
            let len = c.len_utf8();
            assert!(len <= 4);
            if bytes_written.checked_add(len)? > LEN {
                break;
            }

            let mut buf = [0; 4];
            c.encode_utf8(&mut buf);
            let utf8_buf = buf.get(..len)?;

            let start = bytes_written;
            let end = start.checked_add(utf8_buf.len())?;
            bytes[start..end].copy_from_slice(utf8_buf);

            bytes_written = bytes_written.checked_add(utf8_buf.len())?;
        }

        let len = NonZeroUsize::new(bytes_written)?;

        Some(Self { len, bytes })
    }

    #[must_use]
    pub fn new(s: &str) -> Option<Self> {
        let len = NonZeroUsize::new(s.len())?;
        let mut bytes = [0; LEN];
        bytes.get_mut(0..len.get())?.copy_from_slice(s.as_bytes());
        Some(Self { len, bytes })
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len.get()]
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes()).unwrap_or_else(|_| unreachable!())
    }

    #[must_use]
    pub const fn len(&self) -> NonZeroUsize {
        self.len
    }
}

impl<const LEN: usize> core::fmt::Debug for NonEmptyInlineString<LEN> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

impl<const LEN: usize> core::fmt::Display for NonEmptyInlineString<LEN> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl<const LEN: usize> PartialEq for NonEmptyInlineString<LEN> {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}
impl<const LEN: usize> Eq for NonEmptyInlineString<LEN> {}

#[cfg(test)]
mod tess {
    use super::*;

    #[test]
    fn test_short() {
        assert_eq!(
            NonEmptyInlineString::<5>::truncate("abcde")
                .unwrap()
                .as_str(),
            "abcde"
        );
    }

    #[test]
    fn test_long() {
        assert_eq!(
            NonEmptyInlineString::<5>::truncate("abcdef")
                .unwrap()
                .as_str(),
            "abcde"
        );

        assert_eq!('Ⴀ'.len_utf8(), 3);
        assert_eq!(
            NonEmptyInlineString::<10>::truncate("ႠႠႠႠ")
                .unwrap()
                .as_str(),
            "ႠႠႠ"
        );

        assert_eq!('🦴'.len_utf8(), 4);
        assert_eq!(
            NonEmptyInlineString::<10>::truncate("🦴🦴🦴")
                .unwrap()
                .as_str(),
            "🦴🦴"
        );
    }

    #[test]
    fn test_empty() {
        assert_eq!(NonEmptyInlineString::<100>::truncate(""), None);
    }
}
