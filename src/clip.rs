use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct Clip {
    pub(crate) text: String,
    pub(crate) timestamp: u128,
}

impl Clip {
    pub(crate) fn newer_than(&self, other: &Clip) -> bool {
        self.timestamp > other.timestamp && self.text != other.text
    }
}
