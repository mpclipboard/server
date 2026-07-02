use anyhow::{Result, bail};
use std::os::fd::AsRawFd;
use tokio::io::unix::AsyncFd;
use wl_data_control_protocol_evt::{ExtDataControlEvent, ExtDataControlStream};

pub(crate) struct AsyncExtDataControlStream {
    inner: ExtDataControlStream,
    async_fd: AsyncFd<i32>,
}

impl AsyncExtDataControlStream {
    pub(crate) fn new() -> Result<Self> {
        let stream = ExtDataControlStream::new()?;
        let async_fd = AsyncFd::new(stream.as_raw_fd())?;
        Ok(Self {
            inner: stream,
            async_fd,
        })
    }

    pub(crate) fn offer_text(&mut self, text: String) -> Result<()> {
        self.inner.offer_text(text)?;
        Ok(())
    }

    pub(crate) async fn drain(&mut self) -> Result<Option<String>> {
        let mut guard = self.async_fd.readable().await?;
        let events = self.inner.drain()?;
        guard.clear_ready();

        let mut text_to_return = None;
        for event in events {
            match event {
                ExtDataControlEvent::Received(text) => text_to_return = Some(text),
                ExtDataControlEvent::Finished => {
                    bail!("Received Finished event from local clipboard")
                }
            }
        }
        Ok(text_to_return)
    }
}
