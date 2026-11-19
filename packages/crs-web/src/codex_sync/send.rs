use crate::{CodexConnection, CodexConnectionRequestError, CodexSync, Route, ThreadHistory, ThreadRecord, ThreadStoreRecordError, ThreadStoreWriteError, UrlSegment};
use codex_app_server_protocol::{ClientRequest, RequestId, ThreadResumeParams, ThreadResumeResponse, ThreadStartParams, ThreadStartResponse, TurnStartParams, TurnStartResponse, UserInput};
use errgonomic::{handle, handle_bool};
use thiserror::Error;
use tokio::task::{JoinError, spawn_blocking};
use uuid::Uuid;

impl CodexSync {
    pub async fn send(&self, client: &CodexConnection, record: Option<ThreadRecord>, message: String) -> Result<(Route, String), CodexSyncSendError> {
        use CodexSyncSendError::*;
        let new_thread = record.is_none();
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
                    turn_versions: None,
                    thread: response.thread,
                }
            }
        };
        let id = record.thread.id.clone();
        handle_bool!(!self.config.includes_thread(record.thread.created_at), OutsideCutoff, id);
        handle_bool!(record.thread.can_accept_direct_input == Some(false), DirectInputUnavailable, id);
        if new_thread {
            let guard = self.lock_thread(&id).await;
            let store = self.store.clone();
            let config = self.config.clone();
            let source = self.server_id.clone();
            handle!(
                spawn_blocking(move || {
                    let _guard = guard;
                    if handle!(store.record(&store.snapshot(), &config, &source, &record.thread.id), ReadNewThreadFailed).is_none() {
                        handle!(store.write(&source, record, ThreadHistory::default()), SaveNewThreadFailed);
                    }
                    Ok(())
                })
                .await,
                JoinFailed
            )?;
        }
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
        self.refresh.notify_one();
        Ok((Route::thread(source, id, Some(&response.turn.id)), message_id))
    }
}

#[derive(Error, Debug)]
pub enum CodexSyncSendError {
    #[error("failed to check the new thread in local storage")]
    ReadNewThreadFailed { source: ThreadStoreRecordError },
    #[error("failed to store the new thread")]
    SaveNewThreadFailed { source: ThreadStoreWriteError },
    #[error("new thread storage task failed")]
    JoinFailed { source: JoinError },
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
