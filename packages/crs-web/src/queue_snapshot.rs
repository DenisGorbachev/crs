use crate::{QueuedMessage, content_version};
use serde::Serialize;

#[derive(Serialize)]
pub struct QueueSnapshot {
    pub version: String,
    pub messages: Vec<QueuedMessage>,
}

impl From<Vec<QueuedMessage>> for QueueSnapshot {
    fn from(messages: Vec<QueuedMessage>) -> Self {
        Self {
            version: content_version(&messages),
            messages,
        }
    }
}
