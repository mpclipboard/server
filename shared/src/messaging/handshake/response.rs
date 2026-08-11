use crate::{Decode, Encode};

#[derive(Debug, Clone, Copy)]
pub struct HandshakeResponse;

impl HandshakeResponse {
    pub const BYTES: &[u8] = b"HTTP/1.1 101 Switching Protocols\r\n\
Connection: Upgrade\r\n\
Upgrade: mpclipboard-raw\r\n\
Content-Length: 0\r\n\
\r\n";

    pub const BYTESIZE: usize = Self::BYTES.len();
}

impl Encode<{ HandshakeResponse::BYTESIZE }> for HandshakeResponse {
    fn encode(&self, buf: &mut [u8; HandshakeResponse::BYTESIZE]) {
        buf.copy_from_slice(Self::BYTES);
    }
}

impl Decode<{ HandshakeResponse::BYTESIZE }> for HandshakeResponse {
    type Error = HandshakeResponseDecodeError;

    fn decode(buf: &[u8; HandshakeResponse::BYTESIZE]) -> Result<Self, Self::Error> {
        if buf == Self::BYTES {
            Ok(Self)
        } else {
            Err(HandshakeResponseDecodeError)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HandshakeResponseDecodeError;

impl core::fmt::Display for HandshakeResponseDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "HandshakeResponseDecodeError")
    }
}

impl core::error::Error for HandshakeResponseDecodeError {}
