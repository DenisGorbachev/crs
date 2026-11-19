use crate::{CodexConnection, CodexConnectionRequestError};
use codex_app_server_protocol::{ClientRequest, RequestId, SessionSource, Thread, ThreadReadParams, ThreadReadResponse, ThreadReadState, ThreadSource};
use errgonomic::handle;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct ThreadRecord {
    pub archived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_turn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_versions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_state: Option<ThreadReadState>,
    pub thread: Thread,
}

impl ThreadRecord {
    pub async fn refresh_metadata(&mut self, client: &CodexConnection) -> Result<bool, ThreadRecordRefreshMetadataError> {
        use ThreadRecordRefreshMetadataError::*;
        let id = &self.thread.id;
        let response: ThreadReadResponse = handle!(client.request(ClientRequest::ThreadRead {
            request_id: RequestId::String(Uuid::new_v4().to_string()),
            params: ThreadReadParams { thread_id: id.to_owned(), include_turns: false },
        }).await, RequestFailed, id: id.as_str());
        let changed = self.thread != response.thread || self.read_state != response.read_state;
        self.thread = response.thread;
        self.read_state = response.read_state;
        Ok(changed)
    }

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

#[derive(Error, Debug)]
pub enum ThreadRecordRefreshMetadataError {
    #[error("failed to read current metadata for thread '{id}'")]
    RequestFailed { source: CodexConnectionRequestError, id: String },
}
