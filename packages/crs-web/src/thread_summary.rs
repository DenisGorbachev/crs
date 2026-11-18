use crate::ThreadRecord;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ThreadSummary {
    pub id: String,
    pub server_id: String,
    pub archived: bool,
    pub title: String,
    pub updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_turn_id: Option<String>,
}

impl From<(&str, ThreadRecord)> for ThreadSummary {
    fn from((server_id, record): (&str, ThreadRecord)) -> Self {
        let title = record.title().to_owned();
        Self {
            id: record.thread.id,
            server_id: server_id.to_owned(),
            archived: record.archived,
            title,
            updated_at: record.thread.updated_at,
            last_turn_id: record.last_turn_id,
        }
    }
}
