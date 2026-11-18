use codex_app_server_protocol::{SessionSource, Thread, ThreadReadState, ThreadSource};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ThreadRecord {
    pub archived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_turn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_state: Option<ThreadReadState>,
    pub thread: Thread,
}

impl ThreadRecord {
    pub fn is_parent(&self) -> bool {
        self.thread.parent_thread_id.is_none() && self.thread.attached_to_thread_id.is_none() && !matches!(self.thread.source, SessionSource::SubAgent(_)) && self.thread.thread_source != Some(ThreadSource::Subagent)
    }

    pub fn title(&self) -> &str {
        self.thread
            .name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| if self.thread.preview.trim().is_empty() { &self.thread.id } else { &self.thread.preview })
    }
}
