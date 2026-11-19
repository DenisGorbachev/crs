use codex_app_server_protocol::QueuedSubmission;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct PendingSteer {
    pub item: QueuedSubmission,
    pub turn_id: String,
    pub confirmed: bool,
}
