use anyhow::{Context as _, Result};
use http::Response;
use tokio::net::TcpStream;
use tokio_websockets::{ClientBuilder, Connector, MaybeTlsStream, WebSocketStream};

pub(crate) async fn new(
    url: &str,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response<()>)> {
    let uri = http::Uri::try_from(url).context("invalid url")?;
    let is_wss = uri.scheme().map(|scheme| scheme.as_str()) == Some("wss");

    let mut builder = ClientBuilder::from_uri(uri);

    let tls_connector = Connector::NativeTls(tokio_native_tls::TlsConnector::from(
        tokio_native_tls::native_tls::TlsConnector::new().unwrap(),
    ));

    if is_wss {
        log::info!("wss protocol detected, enabling TLS");
        builder = builder.connector(&tls_connector)
    }

    let (ws, response) = builder.connect().await?;
    Ok((ws, response))
}
