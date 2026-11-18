use codex_app_server_protocol::{Thread, ThreadReadState};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ThreadRecord {
    pub archived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_state: Option<ThreadReadState>,
    pub thread: Thread,
}
