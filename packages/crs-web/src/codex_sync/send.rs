use crate::{CodexConnection, CodexConnectionRequestError, CodexSync, Route, ThreadRecord, UrlSegment};
use codex_app_server_protocol::{ClientRequest, RequestId, ThreadResumeParams, ThreadResumeResponse, ThreadStartParams, ThreadStartResponse, TurnStartParams, TurnStartResponse, UserInput};
use errgonomic::{ErrorDisplayer, handle, handle_bool};
use std::time::Duration;
use thiserror::Error;
use tokio::time::timeout;
use tracing::warn;
use uuid::Uuid;

impl CodexSync {
    pub async fn send(&self, client: &CodexConnection, record: Option<ThreadRecord>, message: String) -> Result<Route, CodexSyncSendError> {
        use CodexSyncSendError::*;
        let record = match record {
            Some(mut record) => {
                let response = handle!(client.request::<ThreadResumeResponse>(ClientRequest::ThreadResume {
                    request_id: RequestId::String(Uuid::new_v4().to_string()),
                    params: ThreadResumeParams { thread_id: record.thread.id.clone(), exclude_turns: true, ..Default::default() },
                }).await, ResumeFailed, id: record.thread.id);
                handle_bool!(response.thread.id != record.thread.id, ThreadIdInvalid, id: record.thread.id);
                record.thread = response.thread;
                record
            }
            None => {
                let response = handle!(
                    client
                        .request::<ThreadStartResponse>(ClientRequest::ThreadStart {
                            request_id: RequestId::String(Uuid::new_v4().to_string()),
                            params: ThreadStartParams::default(),
                        })
                        .await,
                    StartThreadFailed
                );
                ThreadRecord {
                    archived: false,
                    read_state: None,
                    last_turn_id: None,
                    thread: response.thread,
                }
            }
        };
        let id = record.thread.id.as_str();
        handle_bool!(!self.config.includes_thread(record.thread.created_at), OutsideCutoff, id);
        handle_bool!(record.thread.can_accept_direct_input == Some(false), DirectInputUnavailable, id);
        let message_id = Uuid::new_v4().to_string();
        let response = handle!(
            client
                .request::<TurnStartResponse>(ClientRequest::TurnStart {
                    request_id: RequestId::String(Uuid::new_v4().to_string()),
                    params: TurnStartParams {
                        thread_id: id.to_owned(),
                        client_user_message_id: Some(message_id.clone()),
                        input: vec![UserInput::Text {
                            text: message,
                            text_elements: Vec::new()
                        }],
                        ..Default::default()
                    },
                })
                .await,
            StartTurnFailed,
            id
        );
        let source = UrlSegment {
            value: self.server_id.clone(),
        };
        let id = id.to_owned();
        let turn = response.turn.id;
        self.refresh.notify_one();
        let refresh = self.save_thread(client, record, Some((&turn, &message_id)));
        match timeout(Duration::from_secs(3), refresh).await {
            Ok(Ok(())) => return Ok(Route::thread(source, id, Some(&turn))),
            Ok(Err(error)) => warn!(server_id = self.server_id, %id, error = %ErrorDisplayer(&error), "accepted message awaits synchronization"),
            Err(_) => warn!(server_id = self.server_id, %id, "accepted message awaits synchronization"),
        }
        let route = Route::MessageSent {
            source,
            id: UrlSegment {
                value: id,
            },
            turn: UrlSegment {
                value: turn,
            },
        };
        Ok(route)
    }
}

#[derive(Error, Debug)]
pub enum CodexSyncSendError {
    #[error("Failed to start a Codex thread.")]
    StartThreadFailed { source: CodexConnectionRequestError },
    #[error("Failed to resume thread '{id}'.")]
    ResumeFailed { source: CodexConnectionRequestError, id: String },
    #[error("Codex resumed a different thread than '{id}'.")]
    ThreadIdInvalid { id: String },
    #[error("Thread '{id}' is outside the configured cutoff.")]
    OutsideCutoff { id: String },
    #[error("Thread '{id}' cannot accept direct input.")]
    DirectInputUnavailable { id: String },
    #[error("No acknowledgement for the message to thread '{id}'. Check the thread before resending.")]
    StartTurnFailed { source: CodexConnectionRequestError, id: String },
}
