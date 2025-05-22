use crate::Clip;
use anyhow::Context as _;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_websockets::Message;

#[derive(Serialize, Deserialize, Clone)]
pub enum Payload {
    Auth { name: String, token: String },
    AuthResult { success: bool },
    Clip(Clip),
}

impl std::fmt::Debug for Payload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auth { name, .. } => {
                write!(f, "Auth {{ name: {name:?}, token: \"*****\" }}")
            }
            Self::AuthResult { success } => {
                write!(f, "AuthResult {{ success: {success} }}")
            }
            Self::Clip(Clip { text, timestamp }) => {
                write!(f, "Clip {{ text: {text:?}, timestamp: {timestamp} }}")
            }
        }
    }
}

impl Payload {
    pub fn auth(name: impl Into<String>, token: impl Into<String>) -> Self {
        Self::Auth {
            name: name.into(),
            token: token.into(),
        }
    }

    pub fn auth_result(success: bool) -> Self {
        Self::AuthResult { success }
    }

    pub fn clip(text: impl Into<String>) -> Self {
        Self::Clip(Clip {
            text: text.into(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs(),
        })
    }
}

impl TryFrom<Message> for Payload {
    type Error = anyhow::Error;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        let text = message.as_text().context("not a text")?;
        let payload: Payload = serde_json::from_str(text).context("malformed message")?;
        Ok(payload)
    }
}

impl From<Payload> for Message {
    fn from(value: Payload) -> Self {
        Message::text(serde_json::to_string(&value).unwrap())
    }
}
