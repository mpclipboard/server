use anyhow::Context as _;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_websockets::Message;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Clip {
    pub text: String,
    pub timestamp: u64,
}

impl Clip {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs(),
        }
    }
}

impl From<Clip> for Message {
    fn from(clip: Clip) -> Self {
        Message::text(serde_json::to_string(&clip).unwrap())
    }
}

impl TryFrom<Message> for Clip {
    type Error = anyhow::Error;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        let text = message.as_text().context("not a text message")?;
        let clip: Clip = serde_json::from_str(text).context("malformed json message")?;
        Ok(clip)
    }
}
