use crate::{CodexAppServerConnectError, CodexConnection, CodexSyncChangeQueueError, CodexSyncSendError, MessageAction, QueuedMessage, Route, SharedServer, ThreadStoreRecordError, UrlSegment};
use axum::http::{HeaderMap, StatusCode, header::ACCEPT};
use axum::response::{IntoResponse, Redirect, Response};
use axum::{Extension, Form, Json};
use errgonomic::{ErrorDisplayer, handle, handle_bool, handle_opt};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use tokio::task::{JoinError, spawn_blocking};
use tracing::warn;

#[derive(Serialize, Deserialize)]
pub struct MessageSubmission {
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(default)]
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<MessageAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queued_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original: Option<String>,
    pub form_key: String,
}

pub async fn submit_message(Extension(server): Extension<SharedServer>, headers: HeaderMap, Form(form): Form<MessageSubmission>) -> Response {
    let result = match tokio::spawn(form.send(server)).await {
        Ok(result) => result,
        Err(source) => Err(MessageSubmissionSendError::JoinFailed {
            source,
        }),
    };
    match result {
        Ok((_, receipt))
            if headers
                .get(ACCEPT)
                .is_some_and(|header| header == "application/json") =>
        {
            Json(receipt).into_response()
        }
        Ok((route, _)) => Redirect::to(&route.to_string()).into_response(),
        Err(error) => {
            let status = error.status();
            if status.is_server_error() {
                warn!(error = %ErrorDisplayer(&error), "Codex message submission failed");
            }
            (status, ErrorDisplayer(&error).to_string()).into_response()
        }
    }
}

impl MessageSubmission {
    pub async fn send(self, server: SharedServer) -> Result<(Route, Option<QueuedMessage>), MessageSubmissionSendError> {
        use MessageSubmissionSendError::*;
        let (sync, db) = {
            let server = server.lock().await;
            handle_bool!(self.form_key != server.form_key.to_string(), FormKeyInvalid);
            let source = handle_opt!(server.sources.get(&self.source), SourceNotFound);
            (source.clone(), server.db.clone())
        };
        handle_bool!(matches!(self.action, None | Some(MessageAction::Edit)) && self.message.trim().is_empty(), MessageEmpty);
        let record = match self.thread_id.clone() {
            Some(id) => {
                let sync = sync.clone();
                let result = handle!(
                    spawn_blocking(move || sync
                        .store
                        .record(&db.snapshot(), &sync.config, &sync.server_id, &id))
                    .await,
                    JoinFailed
                );
                Some(handle_opt!(handle!(result, RecordFailed), ThreadNotFound))
            }
            None => {
                handle_bool!(
                    !sync
                        .config
                        .includes_thread(OffsetDateTime::now_utc().unix_timestamp()),
                    OutsideCutoff
                );
                None
            }
        };
        let client = handle!(sync.source.connect().await, ConnectFailed);
        let connection = CodexConnection::from(&client);
        let result = async {
            let Some(action) = self.action else {
                return Ok(handle!(sync.send(&connection, record, self.message).await, SendFailed));
            };
            let record = handle_opt!(record, ThreadNotFound);
            handle_bool!(record.thread.can_accept_direct_input == Some(false), ReadOnly);
            let queued_id = handle_opt!(self.queued_id, QueueIdMissing);
            let id = record.thread.id;
            let _guard = sync.lock_thread(&id).await;
            let result = sync
                .change_queue(&connection, &id, queued_id, action, self.original.as_deref(), self.message)
                .await;
            sync.viewed_changed.notify_one();
            let updated = handle!(result, ChangeQueueFailed);
            Ok((
                Route::thread(
                    UrlSegment {
                        value: self.source.clone(),
                    },
                    id,
                    record.last_turn_id.as_deref(),
                ),
                updated,
            ))
        }
        .await;
        if let Some(id) = self.thread_id.as_deref()
            && sync.is_viewed(id).await
            && let Err(error) = sync.refresh_queue(&connection, id).await
        {
            if let Some(sender) = sync.viewed.lock().await.get(id) {
                sender.send_replace(None);
            }
            warn!(%id, error = %ErrorDisplayer(&error), "could not refresh queue after submission");
        }
        drop(connection);
        if let Err(error) = client.shutdown().await {
            warn!(server_id = self.source, %error, "failed to close Codex submission connection");
        }
        result
    }
}

#[derive(Error, Debug)]
pub enum MessageSubmissionSendError {
    #[error("This form key does not match this CRS-Web instance. Reload the page before sending.")]
    FormKeyInvalid {},
    #[error("Enter a message before sending.")]
    MessageEmpty {},
    #[error("Codex server not found.")]
    SourceNotFound {},
    #[error("Thread not found.")]
    ThreadNotFound {},
    #[error("New threads are outside the configured cutoff.")]
    OutsideCutoff {},
    #[error("Message submission task failed.")]
    JoinFailed { source: JoinError },
    #[error("Could not read the saved thread.")]
    RecordFailed { source: ThreadStoreRecordError },
    #[error("Could not connect to the Codex server.")]
    ConnectFailed { source: CodexAppServerConnectError },
    #[error("This thread cannot accept direct input.")]
    ReadOnly {},
    #[error("Missing queued message ID.")]
    QueueIdMissing {},
    #[error("Could not change the queued message.")]
    ChangeQueueFailed { source: CodexSyncChangeQueueError },
    #[error("Could not submit the message.")]
    SendFailed { source: CodexSyncSendError },
}

impl MessageSubmissionSendError {
    fn status(&self) -> StatusCode {
        use MessageSubmissionSendError::*;
        match self {
            ChangeQueueFailed {
                source,
            } => source.status(),
            ReadOnly {} => StatusCode::CONFLICT,
            FormKeyInvalid {} => StatusCode::FORBIDDEN,
            MessageEmpty {} | OutsideCutoff {} | QueueIdMissing {} => StatusCode::UNPROCESSABLE_ENTITY,
            SourceNotFound {} | ThreadNotFound {} => StatusCode::NOT_FOUND,
            RecordFailed {
                ..
            }
            | JoinFailed {
                ..
            } => StatusCode::INTERNAL_SERVER_ERROR,
            SendFailed {
                source: CodexSyncSendError::DirectInputUnavailable {
                    ..
                },
            } => StatusCode::CONFLICT,
            SendFailed {
                source: CodexSyncSendError::OutsideCutoff {
                    ..
                },
            } => StatusCode::NOT_FOUND,
            ConnectFailed {
                ..
            }
            | SendFailed {
                ..
            } => StatusCode::BAD_GATEWAY,
        }
    }
}
