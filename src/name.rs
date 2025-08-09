use anyhow::{Result, ensure};

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub(crate) struct Name {
    bytes: [u8; 20],
    len: usize,
}

impl Name {
    pub(crate) fn new(name: String) -> Result<Self> {
        let len = name.len();
        let bytes = name.into_bytes();
        ensure!(len <= 20, "Name is too long: {len} > 20");

        let mut out = [0_u8; 20];
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
