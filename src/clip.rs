use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use tokio_websockets::Message;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct Clip {
    pub(crate) text: String,
    pub(crate) timestamp: u128,
}

impl Clip {
    pub(crate) fn to_message(&self) -> Message {
        Message::text(serde_json::to_string(self).unwrap())
    }

    pub(crate) fn from_message(message: &Message) -> Result<Self> {
        let text = message.as_text().context("not a text message")?;
        let clip: Clip = serde_json::from_str(text).context("malformed json message")?;
        Ok(clip)
    }
}
