use crate::{
    CONNECTION_UPGRADE_HEADER, Decode, Encode, HOST_PREFIX, Host, ID, ID_PREFIX, MAX_HOST_LENGTH,
    MAX_ID_LENGTH, MAX_TOKEN_LENGTH, MIN_PADDING_LENGTH, NonEmptyInlineString, PADDING_PREFIX,
    START_LINE, TOKEN_PREFIX, Token, UPGRADE_MPCLIPBOARD_RAW_HEADER,
    strip_prefix_ignore_ascii_case,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandshakeRequest {
    pub host: Host,
    pub token: Token,
    pub id: ID,
}

const _IGNORE: () = assert!(HandshakeRequest::BYTESIZE == 562);

const BASE_HANDSHAKE_LENGTH: usize = START_LINE.len() + 2 // start line
    + HOST_PREFIX.len() + 2 // Host: ...
    + TOKEN_PREFIX.len() + 2 // Token: ...
    + ID_PREFIX.len() + 2 // ID: ...
    + CONNECTION_UPGRADE_HEADER.len() + 2 // Connection: Upgrade
    + UPGRADE_MPCLIPBOARD_RAW_HEADER.len() + 2 // Upgrade: mpclipboard-raw
    + PADDING_PREFIX.len() + 2 //
    + 2; // headers end marker

const EXPECTED_LINES_COUNT: usize = 8;

impl HandshakeRequest {
    pub const BYTESIZE: usize = BASE_HANDSHAKE_LENGTH
        + MAX_HOST_LENGTH
        + MAX_TOKEN_LENGTH
        + MAX_ID_LENGTH
        + MIN_PADDING_LENGTH;
}

impl Encode<{ HandshakeRequest::BYTESIZE }> for HandshakeRequest {
    fn encode(&self, buf: &mut [u8; HandshakeRequest::BYTESIZE]) {
        let mut pos = 0;

        macro_rules! append {
            ($s:expr) => {
                buf[pos..pos + $s.len()].copy_from_slice($s.as_bytes());
                pos += $s.len();
            };
        }

        append!(START_LINE);
        append!("\r\n");

        append!(HOST_PREFIX);
        append!(self.host.as_str());
        append!("\r\n");

        append!(TOKEN_PREFIX);
        append!(self.token.as_str());
        append!("\r\n");

        append!(ID_PREFIX);
        append!(self.id.as_str());
        append!("\r\n");

        append!(CONNECTION_UPGRADE_HEADER);
        append!("\r\n");

        append!(UPGRADE_MPCLIPBOARD_RAW_HEADER);
        append!("\r\n");

        append!(PADDING_PREFIX);
        #[expect(clippy::arithmetic_side_effects)]
        let padding_len = Self::BYTESIZE - 4 - pos;
        for _ in 0..padding_len {
            append!("P");
        }
        append!("\r\n");

        append!("\r\n");

        assert_eq!(pos, Self::BYTESIZE);
    }
}

impl Decode<{ HandshakeRequest::BYTESIZE }> for HandshakeRequest {
    type Error = HandshakeRequestDecodeError;

    fn decode(buf: &[u8; HandshakeRequest::BYTESIZE]) -> Result<Self, Self::Error> {
        let buf = core::str::from_utf8(buf).map_err(|_| HandshakeRequestDecodeError::NotUtf8)?;

        let mut lines = buf.lines();
        let lines_count = buf
            .bytes()
            .zip(buf.bytes().skip(1))
            .filter(|(prev, next)| *prev == b'\r' && *next == b'\n')
            .count();

        if lines_count != EXPECTED_LINES_COUNT {
            return Err(HandshakeRequestDecodeError::NotEnoughLines {
                actual: lines_count,
                expected: EXPECTED_LINES_COUNT,
            });
        }

        macro_rules! next_line {
            () => {
                lines
                    .next()
                    .ok_or(HandshakeRequestDecodeError::NotEnoughLines {
                        actual: lines_count,
                        expected: EXPECTED_LINES_COUNT,
                    })?
            };
        }

        if !next_line!().eq_ignore_ascii_case(START_LINE) {
            return Err(HandshakeRequestDecodeError::MalformedStatusLine);
        }

        let host = strip_prefix_ignore_ascii_case(next_line!(), HOST_PREFIX)
            .and_then(NonEmptyInlineString::new)
            .ok_or(HandshakeRequestDecodeError::MalformedHostHeader)?;

        let token = strip_prefix_ignore_ascii_case(next_line!(), TOKEN_PREFIX)
            .and_then(NonEmptyInlineString::new)
            .ok_or(HandshakeRequestDecodeError::MalformedTokenHeader)?;

        let id = strip_prefix_ignore_ascii_case(next_line!(), ID_PREFIX)
            .and_then(NonEmptyInlineString::new)
            .ok_or(HandshakeRequestDecodeError::MalformedIdHeader)?;

        if !next_line!().eq_ignore_ascii_case(CONNECTION_UPGRADE_HEADER) {
            return Err(HandshakeRequestDecodeError::MalformedConnectionUpgradeHeader);
        }

        if !next_line!().eq_ignore_ascii_case(UPGRADE_MPCLIPBOARD_RAW_HEADER) {
            return Err(HandshakeRequestDecodeError::MalformedUpgradeMpclipboardRawHeader);
        }

        let padding = strip_prefix_ignore_ascii_case(next_line!(), PADDING_PREFIX)
            .ok_or(HandshakeRequestDecodeError::MalformedPaddingHeader)?;
        if padding.bytes().any(|b| b != b'P' && b != b'p') {
            return Err(HandshakeRequestDecodeError::MalformedPaddingHeader);
        }

        if !next_line!().is_empty() {
            return Err(HandshakeRequestDecodeError::MalformedEmptyTrailingLine);
        }

        Ok(Self { host, token, id })
    }
}

#[expect(missing_docs)]
#[derive(Debug, Clone, Copy)]
pub enum HandshakeRequestDecodeError {
    NotUtf8,
    NotEnoughLines { actual: usize, expected: usize },
    MalformedStatusLine,
    MalformedHostHeader,
    MalformedTokenHeader,
    MalformedIdHeader,
    MalformedConnectionUpgradeHeader,
    MalformedUpgradeMpclipboardRawHeader,
    MalformedPaddingHeader,
    MalformedEmptyTrailingLine,
}

impl core::fmt::Display for HandshakeRequestDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotUtf8 => write!(f, "NotUtf8"),
            Self::NotEnoughLines { actual, expected } => f
                .debug_struct("NotEnoughLines")
                .field("actual", actual)
                .field("expected", expected)
                .finish(),
            Self::MalformedStatusLine => write!(f, "MalformedStatusLine"),
            Self::MalformedHostHeader => write!(f, "MalformedHostHeader"),
            Self::MalformedTokenHeader => write!(f, "MalformedTokenHeader"),
            Self::MalformedIdHeader => write!(f, "MalformedIdHeader"),
            Self::MalformedConnectionUpgradeHeader => write!(f, "MalformedConnectionUpgradeHeader"),
            Self::MalformedUpgradeMpclipboardRawHeader => {
                write!(f, "MalformedUpgradeMpclipboardRawHeader")
            }
            Self::MalformedPaddingHeader => write!(f, "MalformedPaddingHeader"),
            Self::MalformedEmptyTrailingLine => write!(f, "MalformedEmptyTrailingLine"),
        }
    }
}

impl core::error::Error for HandshakeRequestDecodeError {}
