use anyhow::{Context as _, Result, bail};
use codec::Codec;
use futures_util::Stream as _;
use std::{future::poll_fn, pin::Pin};
use tokio::{io::AsyncWriteExt as _, net::TcpStream};
use tokio_util::codec::FramedRead;
use tokio_websockets::{ServerBuilder, WebSocketStream};

mod codec;

pub(crate) struct Handshake {
    token: String,
    name: String,
    response_body_on_accept: String,
    stream: TcpStream,
}

impl Handshake {
    const BAD_REQUEST: &[u8] = b"HTTP/1.1 400 Bad Request\r\n\r\n";
    const UNAUTHORIZED: &[u8] = b"HTTP/1.1 401 Unauthorized\r\n\r\n";

    pub(crate) async fn parse(stream: TcpStream) -> Result<Handshake> {
        let mut framed = FramedRead::new(stream, Codec);
        let frame = poll_fn(|cx| Pin::new(&mut framed).poll_next(cx)).await;
        let frame = frame.context("EOF error")?;

        let (request, response_body_on_accept) = match frame {
            Ok(frame) => frame,
            Err(err) => {
                framed.get_mut().write_all(Self::BAD_REQUEST).await?;
                return Err(err);
            }
        };

        let header = |header: &str| -> Result<String> {
            Ok(request
                .headers()
                .get(header)
                .with_context(|| format!("no {header} header"))?
                .to_str()
                .with_context(|| format!("malformed {header} header"))?
                .to_string())
        };
        let token = header("token")?;
        let name = header("name")?;

        Ok(Handshake {
            token,
            name,
            response_body_on_accept,
            stream: framed.into_inner(),
        })
    }

    pub(crate) async fn authenticate(&mut self, token: &str) -> Result<()> {
        if token == self.token {
            log::info!("[auth] {} OK", self.name);
            Ok(())
        } else {
            log::error!("[auth] {} FAILED", self.name);
            self.stream
                .write_all(Self::UNAUTHORIZED)
                .await
                .context("failed to write response back")?;
            bail!("auth failed");
        }
    }

    pub(crate) async fn accept(mut self) -> Result<(String, WebSocketStream<TcpStream>)> {
        self.stream
            .write_all(self.response_body_on_accept.as_bytes())
            .await
            .context("failed to write response back")?;

        let ws = ServerBuilder::new().serve(self.stream);

        Ok((self.name, ws))
    }
}
