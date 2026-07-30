use anyhow::{Result, bail};
use rustix::event::{PollFd, PollFlags};
use wl_data_control_protocol_evt::{ExtDataControlEvent, ExtDataControlStream};

pub(crate) struct LocalClipboared {
    stream: ExtDataControlStream,
}

impl LocalClipboared {
    pub(crate) fn new() -> Result<Self> {
        let stream = ExtDataControlStream::new()?;
        Ok(Self { stream })
    }

    pub(crate) fn as_pollfd(&self) -> PollFd<'_> {
        PollFd::new(&self.stream, PollFlags::IN)
    }

    pub(crate) fn offer_text(&mut self, text: String) -> Result<()> {
        self.stream.offer_text(text)?;
        Ok(())
    }

    pub(crate) fn read(&mut self) -> Result<Option<String>> {
        let events = self.stream.drain()?;

        let mut text_to_return = None;
        for event in events {
            match event {
                ExtDataControlEvent::Received(text) => {
                    if !text.is_empty() {
                        text_to_return = Some(text);
                    }
                }
                ExtDataControlEvent::Finished => {
                    bail!("Received Finished event from local clipboard")
                }
            }
        }
        Ok(text_to_return)
    }
}
