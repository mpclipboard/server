use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Clip {
    pub text: String,
    pub timestamp: u64,
}
