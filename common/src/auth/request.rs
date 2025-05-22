use anyhow::Context as _;
use serde::{Deserialize, Serialize};
use tokio_websockets::Message;

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub name: String,
    pub token: String,
}

impl Request {
    pub fn new(name: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            token: token.into(),
        }
    }
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Request")
            .field("name", &self.name)
            .field("token", &"*****")
            .finish()
    }
}

impl From<Request> for Message {
    fn from(request: Request) -> Self {
        Message::text(serde_json::to_string(&request).unwrap())
    }
}

impl TryFrom<Message> for Request {
    type Error = anyhow::Error;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        let text = message.as_text().context("not a text message")?;
        let request: Request = serde_json::from_str(text).context("malformed message json")?;
        Ok(request)
    }
}
