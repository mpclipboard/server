use crate::{Decode, Encode};

#[derive(Debug, Clone, Copy)]
pub struct HeartbeatResponse;

impl HeartbeatResponse {
    pub const BYTES: &[u8] = b"HTTP/1.1 101 Switching Protocols\r\n\
Connection: Upgrade\r\n\
Upgrade: mpclipboard-raw\r\n\
Content-Length: 0\r\n\
\r\n";

    pub const BYTESIZE: usize = Self::BYTES.len();
}

impl Encode<{ HeartbeatResponse::BYTESIZE }> for HeartbeatResponse {
    fn encode(&self, buf: &mut [u8; HeartbeatResponse::BYTESIZE]) {
        buf.copy_from_slice(Self::BYTES);
    }
}

impl Decode<{ HeartbeatResponse::BYTESIZE }> for HeartbeatResponse {
    type Error = HeartbeatResponseDecodeError;

    fn decode(buf: &[u8; HeartbeatResponse::BYTESIZE]) -> Result<Self, Self::Error> {
        if buf == Self::BYTES {
            Ok(Self)
        } else {
            Err(HeartbeatResponseDecodeError)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HeartbeatResponseDecodeError;

impl core::fmt::Display for HeartbeatResponseDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "HeartbeatResponseDecodeError")
    }
}

impl core::error::Error for HeartbeatResponseDecodeError {}
