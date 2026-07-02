use crate::{clipboard::AsyncExtDataControlStream, mpclipboard::MPClipboardStream, tray::Tray};
use anyhow::{Context as _, Result};
use mpclipboard_generic_client::Output;
use std::time::Duration;
use tokio::{
    signal::unix::{Signal, SignalKind},
    time::timeout,
};
use tokio_util::sync::CancellationToken;

pub(crate) struct MainLoop {
    token: CancellationToken,
    mpclipboard: MPClipboardStream,
    tray: Tray,
    clipboard: AsyncExtDataControlStream,
    sigterm: Signal,
    sigint: Signal,
}

impl MainLoop {
    pub(crate) async fn new() -> Result<Self> {
        let token = CancellationToken::new();
        let mpclipboard = MPClipboardStream::new()?;
        let tray = Tray::spawn(token.clone()).await?;
        let clipboard = AsyncExtDataControlStream::new()?;
        let sigterm = tokio::signal::unix::signal(SignalKind::terminate())
            .context("failed to construct SIGTERM handler")?;
        let sigint = tokio::signal::unix::signal(SignalKind::interrupt())
            .context("failed to construct SIGINT handler")?;

        Ok(Self {
            token,
            mpclipboard,
            tray,
            clipboard,
            sigterm,
            sigint,
        })
    }

    pub(crate) async fn start(mut self) -> Result<()> {
        loop {
            tokio::select! {
                output = self.mpclipboard.read() => {
                    self.on_output_from_mpclipboard(output?).await?;
                }

                text = self.clipboard.drain() => {
                    if let Some(text) = text? {
                        self.on_text_from_local_clipboard(text).await?;
                    }
                }

                _ = self.sigterm.recv() => self.on_signal("SIGTERM"),
                _ = self.sigint.recv() => self.on_signal("SIGINT"),

                () = self.token.cancelled() => {
                    log::info!("exiting...");
                    break;
                }
            }
        }

        self.stop().await;
        Ok(())
    }

    async fn on_output_from_mpclipboard(&mut self, output: Option<Output>) -> Result<()> {
        let Some(output) = output else { return Ok(()) };

        match output {
            Output::ConnectivityChanged { connectivity } => {
                log::info!(target: "MPClipboard", "connectivity = {connectivity:?}");
                self.tray.set_connectivity(connectivity).await;
            }

            Output::NewText { text } => {
                log::info!(target: "MPClipboard", "new clip {text:?}");
                self.clipboard.offer_text(text.clone())?;
                self.tray.push_received(&text).await;
            }
        }

        Ok(())
    }

    async fn on_text_from_local_clipboard(&mut self, text: String) -> Result<()> {
        log::info!(target: "LocalReader", "{text}");
        if self.mpclipboard.push_text(text.clone())? {
            self.tray.push_sent(&text).await;
        }
        Ok(())
    }

    fn on_signal(&self, signal: &str) {
        log::info!("{signal} received...");
        self.token.cancel();
    }

    pub(crate) async fn stop(self) {
        if timeout(Duration::from_secs(5), self.tray.stop())
            .await
            .is_err()
        {
            log::warn!("Tray shutdown timed out after 5 seconds");
        }
    }
}
