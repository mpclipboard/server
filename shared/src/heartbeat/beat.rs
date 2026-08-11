use crate::{Decode, Encode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Beat;

impl Beat {
    pub const BYTESIZE: usize = 1;
}

impl Encode<{ Beat::BYTESIZE }> for Beat {
    fn encode(&self, buf: &mut [u8; Beat::BYTESIZE]) {
        buf[0] = 0;
    }
}

impl Decode<{ Beat::BYTESIZE }> for Beat {
    type Error = core::convert::Infallible;

    fn decode(_: &[u8; Beat::BYTESIZE]) -> Result<Self, Self::Error> {
        Ok(Self)
    }
}
