use codex_app_server_protocol::{Thread, ThreadReadState};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ThreadRecord {
    pub archived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_state: Option<ThreadReadState>,
    pub thread: Thread,
}

impl ThreadRecord {
    pub fn title(&self) -> &str {
        self.thread
            .name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| if self.thread.preview.trim().is_empty() { &self.thread.id } else { &self.thread.preview })
    }
}
