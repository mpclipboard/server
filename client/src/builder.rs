use anyhow::{Context as _, Result};
use http::Response;
use rustls::ClientConfig;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_websockets::{ClientBuilder, Connector, MaybeTlsStream, WebSocketStream};

pub(crate) async fn new(
    url: &str,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response<()>)> {
    let uri = http::Uri::try_from(url).context("invalid url")?;
    let is_wss = uri.scheme().map(|scheme| scheme.as_str()) == Some("wss");

    let mut builder = ClientBuilder::from_uri(uri);

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let root_store = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));

    let tls_connector = Connector::Rustls(connector);

    if is_wss {
        log::info!("wss protocol detected, enabling TLS");
        builder = builder.connector(&tls_connector)
    }

    let (ws, response) = builder.connect().await?;
    Ok((ws, response))
}
