mod request;
pub use request::{HeartbeatDecodeError, HeartbeatRequest};

mod response;
pub use response::{HeartbeatResponse, HeartbeatResponseDecodeError};

mod beat;
pub use beat::Beat;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Decode, Encode, MAX_HOST_LENGTH, MAX_ID_LENGTH, NonEmptyInlineString};
    use std::assert_matches;

    #[test]
    fn test_encode_min() {
        let min = HeartbeatRequest {
            host: NonEmptyInlineString::new("h").unwrap(),
            id: NonEmptyInlineString::new("i").unwrap(),
        };

        let expected = [
            "GET / HTTP/1.1\r\n",
            "Host: h\r\n",
            "ID: i\r\n",
            "Connection: Upgrade\r\n",
            "Upgrade: mpclipboard-raw\r\n",
            "Padding: PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPP\r\n",
            "\r\n"
        ].join("");

        let mut buf = [0; _];
        min.encode(&mut buf);
        assert_eq!(
            core::str::from_utf8(&buf).map(|s| s.to_string()),
            Ok(expected)
        );
    }

    #[test]
    fn test_encode_max() {
        let max = HeartbeatRequest {
            host: NonEmptyInlineString::new(&"h".repeat(MAX_HOST_LENGTH)).unwrap(),
            id: NonEmptyInlineString::new(&"i".repeat(MAX_ID_LENGTH)).unwrap(),
        };

        let expected = [
            "GET / HTTP/1.1\r\n",
            "Host: hhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh\r\n",
            "ID: iiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiii\r\n",
            "Connection: Upgrade\r\n",
            "Upgrade: mpclipboard-raw\r\n",
            "Padding: P\r\n",
            "\r\n"
        ].join("");

        let mut buf = [0; _];
        max.encode(&mut buf);
        assert_eq!(
            core::str::from_utf8(&buf).map(|s| s.to_string()),
            Ok(expected)
        );
    }

    #[test]
    fn test_decode_ok() {
        let req = HeartbeatRequest {
            host: NonEmptyInlineString::new("h").unwrap(),
            id: NonEmptyInlineString::new("i").unwrap(),
        };

        assert_eq!(
            {
                let mut buf = [0; _];
                req.encode(&mut buf);
                HeartbeatRequest::decode(&buf).unwrap()
            },
            req
        );
    }

    #[test]
    fn test_decode_err() {
        assert_matches!(
            HeartbeatRequest::decode(&[b'x'; _]),
            Err(HeartbeatDecodeError::NotEnoughLines {
                actual: 0,
                expected: 7
            })
        );
    }
}
