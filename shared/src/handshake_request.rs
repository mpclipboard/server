use crate::{
    CONNECTION_UPGRADE_HEADER, HOST_PREFIX, Host, ID, ID_PREFIX, MAX_HOST_LENGTH, MAX_ID_LENGTH,
    MAX_TOKEN_LENGTH, MIN_PADDING_LENGTH, PADDING_PREFIX, START_LINE, TOKEN_PREFIX, Token,
    UPGRADE_MPCLIPBOARD_RAW_HEADER,
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

impl HandshakeRequest {
    pub const BYTESIZE: usize = BASE_HANDSHAKE_LENGTH
        + MAX_HOST_LENGTH
        + MAX_TOKEN_LENGTH
        + MAX_ID_LENGTH
        + MIN_PADDING_LENGTH;
}

impl HandshakeRequest {
    pub(crate) fn encode(&self) -> [u8; Self::BYTESIZE] {
        let mut buf = [0; Self::BYTESIZE];
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
        buf
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        MAX_HOST_LENGTH, MAX_ID_LENGTH, MAX_TOKEN_LENGTH, NonEmptyInlineString,
        handshake_request::HandshakeRequest,
    };

    #[test]
    fn test_encode_min() {
        let min = HandshakeRequest {
            host: NonEmptyInlineString::new("h").unwrap(),
            token: NonEmptyInlineString::new("t").unwrap(),
            id: NonEmptyInlineString::new("i").unwrap(),
        };

        let expected = [
            "GET / HTTP/1.1\r\n",
            "Host: h\r\n",
            "Token: t\r\n",
            "ID: i\r\n",
            "Connection: Upgrade\r\n",
            "Upgrade: mpclipboard-raw\r\n",
            "Padding: PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPP\r\n",
            "\r\n"
        ].join("");

        assert_eq!(
            core::str::from_utf8(&min.encode()).map(|s| s.to_string()),
            Ok(expected)
        );
    }

    #[test]
    fn test_encode_max() {
        let max = HandshakeRequest {
            host: NonEmptyInlineString::new(&"h".repeat(MAX_HOST_LENGTH)).unwrap(),
            token: NonEmptyInlineString::new(&"t".repeat(MAX_TOKEN_LENGTH)).unwrap(),
            id: NonEmptyInlineString::new(&"i".repeat(MAX_ID_LENGTH)).unwrap(),
        };

        let expected = [
            "GET / HTTP/1.1\r\n",
            "Host: hhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh\r\n",
            "Token: tttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttt\r\n",
            "ID: iiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiii\r\n",
            "Connection: Upgrade\r\n",
            "Upgrade: mpclipboard-raw\r\n",
            "Padding: P\r\n",
            "\r\n"
        ].join("");

        assert_eq!(
            core::str::from_utf8(&max.encode()).map(|s| s.to_string()),
            Ok(expected)
        );
    }
}
