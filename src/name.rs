use anyhow::{Result, ensure};

const MAX_LEN: usize = 64;

/// A trivially copyable connection name.
/// Used to identify clients and do better logging.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub(crate) struct Name {
    bytes: [u8; MAX_LEN],
    len: usize,
}

impl Name {
    pub(crate) fn new(name: String) -> Result<Self> {
        let len = name.len();
        let bytes = name.into_bytes();
        ensure!(len <= MAX_LEN, "Name is too long: {len} > {MAX_LEN}");

        let mut out = [0_u8; MAX_LEN];
        for (idx, byte) in bytes.into_iter().enumerate() {
            out[idx] = byte;
        }
        Ok(Self { bytes: out, len })
    }

    pub(crate) fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).expect("Name was created from non-utf8 String")
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Debug for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}
