use crate::{
    CONNECTION_UPGRADE_HEADER, Decode, Encode, HOST_PREFIX, Host, ID, ID_PREFIX, MAX_HOST_LENGTH,
    MAX_ID_LENGTH, MIN_PADDING_LENGTH, NonEmptyInlineString, PADDING_PREFIX, START_LINE,
    UPGRADE_MPCLIPBOARD_RAW_HEADER, strip_prefix_ignore_ascii_case,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeartbeatRequest {
    pub host: Host,
    pub id: ID,
}

const _IGNORE: () = assert!(HeartbeatRequest::BYTESIZE == 453);

const BASE_HANDSHAKE_LENGTH: usize = START_LINE.len() + 2 // start line
    + HOST_PREFIX.len() + 2 // Host: ...
    + ID_PREFIX.len() + 2 // ID: ...
    + CONNECTION_UPGRADE_HEADER.len() + 2 // Connection: Upgrade
    + UPGRADE_MPCLIPBOARD_RAW_HEADER.len() + 2 // Upgrade: mpclipboard-raw
    + PADDING_PREFIX.len() + 2 //
    + 2; // headers end marker

const EXPECTED_LINES_COUNT: usize = 7;

impl HeartbeatRequest {
    pub const BYTESIZE: usize =
        BASE_HANDSHAKE_LENGTH + MAX_HOST_LENGTH + MAX_ID_LENGTH + MIN_PADDING_LENGTH;
}

impl Encode<{ HeartbeatRequest::BYTESIZE }> for HeartbeatRequest {
    fn encode(&self, buf: &mut [u8; HeartbeatRequest::BYTESIZE]) {
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

impl Decode<{ HeartbeatRequest::BYTESIZE }> for HeartbeatRequest {
    type Error = HeartbeatDecodeError;

    fn decode(buf: &[u8; HeartbeatRequest::BYTESIZE]) -> Result<Self, Self::Error> {
        let buf = core::str::from_utf8(buf).map_err(|_| HeartbeatDecodeError::NotUtf8)?;

        let mut lines = buf.lines();
        let lines_count = buf
            .bytes()
            .zip(buf.bytes().skip(1))
            .filter(|(prev, next)| *prev == b'\r' && *next == b'\n')
            .count();

        if lines_count != EXPECTED_LINES_COUNT {
            return Err(HeartbeatDecodeError::NotEnoughLines {
                actual: lines_count,
                expected: EXPECTED_LINES_COUNT,
            });
        }

        macro_rules! next_line {
            () => {
                lines.next().ok_or(HeartbeatDecodeError::NotEnoughLines {
                    actual: lines_count,
                    expected: EXPECTED_LINES_COUNT,
                })?
            };
        }

        if !next_line!().eq_ignore_ascii_case(START_LINE) {
            return Err(HeartbeatDecodeError::MalformedStatusLine);
        }

        let host = strip_prefix_ignore_ascii_case(next_line!(), HOST_PREFIX)
            .and_then(NonEmptyInlineString::new)
            .ok_or(HeartbeatDecodeError::MalformedHostHeader)?;

        let id = strip_prefix_ignore_ascii_case(next_line!(), ID_PREFIX)
            .and_then(NonEmptyInlineString::new)
            .ok_or(HeartbeatDecodeError::MalformedIdHeader)?;

        if !next_line!().eq_ignore_ascii_case(CONNECTION_UPGRADE_HEADER) {
            return Err(HeartbeatDecodeError::MalformedConnectionUpgradeHeader);
        }

        if !next_line!().eq_ignore_ascii_case(UPGRADE_MPCLIPBOARD_RAW_HEADER) {
            return Err(HeartbeatDecodeError::MalformedUpgradeMpclipboardRawHeader);
        }

        let padding = strip_prefix_ignore_ascii_case(next_line!(), PADDING_PREFIX)
            .ok_or(HeartbeatDecodeError::MalformedPaddingHeader)?;
        if padding.bytes().any(|b| b != b'P' && b != b'p') {
            return Err(HeartbeatDecodeError::MalformedPaddingHeader);
        }

        if !next_line!().is_empty() {
            return Err(HeartbeatDecodeError::MalformedEmptyTrailingLine);
        }

        Ok(Self { host, id })
    }
}

#[expect(missing_docs)]
#[derive(Debug, Clone, Copy)]
pub enum HeartbeatDecodeError {
    NotUtf8,
    NotEnoughLines { actual: usize, expected: usize },
    MalformedStatusLine,
    MalformedHostHeader,
    MalformedIdHeader,
    MalformedConnectionUpgradeHeader,
    MalformedUpgradeMpclipboardRawHeader,
    MalformedPaddingHeader,
    MalformedEmptyTrailingLine,
}

impl core::fmt::Display for HeartbeatDecodeError {
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

impl core::error::Error for HeartbeatDecodeError {}
