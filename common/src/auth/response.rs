use anyhow::Context as _;
use serde::{Deserialize, Serialize};
use tokio_websockets::Message;

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    pub success: bool,
}

impl Response {
    pub fn new(success: bool) -> Self {
        Self { success }
    }
}

impl From<Response> for Message {
    fn from(response: Response) -> Self {
        Message::text(serde_json::to_string(&response).unwrap())
    }
}

impl TryFrom<Message> for Response {
    type Error = anyhow::Error;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        let text = message.as_text().context("not a text message")?;
        let response: Response = serde_json::from_str(text).context("malformed jsom message")?;
        Ok(response)
    }
}
