use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use tokio_websockets::Message;

#[derive(Deserialize)]
pub(crate) struct Request {
    pub(crate) token: String,
}

impl Request {
    pub(crate) fn from_message(message: &Message) -> Result<Self> {
        let text = message.as_text().context("not a text message")?;
        serde_json::from_str(text).context("malformed json message")
    }
}

#[derive(Serialize)]
pub(crate) struct Response {
    pub(crate) success: bool,
}
