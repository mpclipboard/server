pub mod request;
pub mod response;

#[cfg(test)]
mod tests {
    use crate::{
        Encode, MAX_HOST_LENGTH, MAX_ID_LENGTH, MAX_TOKEN_LENGTH, NonEmptyInlineString,
        messaging::handshake::request::HandshakeRequest,
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

        let mut buf = [0; _];
        min.encode(&mut buf);
        assert_eq!(
            core::str::from_utf8(&buf).map(|s| s.to_string()),
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

        let mut buf = [0; _];
        max.encode(&mut buf);
        assert_eq!(
            core::str::from_utf8(&buf).map(|s| s.to_string()),
            Ok(expected)
        );
    }

    // #[test]
    // fn test_decode_ok() {
    //     let req = HandshakeRequest {
    //         host: NonEmptyInlineString::new("h").unwrap(),
    //         token: NonEmptyInlineString::new("t").unwrap(),
    //         id: NonEmptyInlineString::new("i").unwrap(),
    //     };

    //     assert_eq!(
    //         {
    //             let mut buf = [0; _];
    //             req.encode(&mut buf);
    //             HandshakeRequest::decode(&buf).unwrap()
    //         },
    //         req
    //     );
    // }

    // #[test]
    // fn test_decode_err() {
    //     assert_matches!(
    //         HandshakeRequest::decode(&[b'x'; _]),
    //         Err(HandshakeRequestDecodeError::MalformedHostHeader)
    //     );
    // }
}
