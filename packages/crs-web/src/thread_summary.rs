use crate::ThreadRecord;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ThreadSummary {
    pub id: String,
    pub server_id: String,
    pub archived: bool,
    pub title: String,
    pub updated_at: i64,
}

impl From<(&str, ThreadRecord)> for ThreadSummary {
    fn from((server_id, record): (&str, ThreadRecord)) -> Self {
        let thread = record.thread;
        let title = thread
            .name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| if thread.preview.trim().is_empty() { thread.id.clone() } else { thread.preview });
        Self {
            id: thread.id,
            server_id: server_id.to_owned(),
            archived: record.archived,
            title,
            updated_at: thread.updated_at,
        }
    }
}
