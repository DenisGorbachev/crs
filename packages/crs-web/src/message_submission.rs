use crate::{CodexAppServerConnectError, CodexConnection, CodexSyncSendError, Route, SharedServer, ThreadStoreRecordError};
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
    pub message: String,
    pub form_key: String,
}

pub async fn submit_message(Extension(server): Extension<SharedServer>, headers: HeaderMap, Form(form): Form<MessageSubmission>) -> Response {
    match form.send(server).await {
        Ok((_, message_id))
            if headers
                .get(ACCEPT)
                .is_some_and(|header| header == "application/json") =>
        {
            Json(message_id).into_response()
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
    pub async fn send(self, server: SharedServer) -> Result<(Route, String), MessageSubmissionSendError> {
        use MessageSubmissionSendError::*;
        let (sync, db) = {
            let server = server.lock().await;
            handle_bool!(self.form_key != server.form_key.to_string(), FormKeyInvalid);
            let source = handle_opt!(server.sources.get(&self.source), SourceNotFound);
            (source.clone(), server.db.clone())
        };
        handle_bool!(self.message.trim().is_empty(), MessageEmpty);
        let record = match self.thread_id {
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
        let result = sync.send(&connection, record, self.message).await;
        drop(connection);
        if let Err(error) = client.shutdown().await {
            warn!(server_id = self.source, %error, "failed to close Codex submission connection");
        }
        Ok(handle!(result, SendFailed))
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
    #[error("Thread storage task failed.")]
    JoinFailed { source: JoinError },
    #[error("Could not read the saved thread.")]
    RecordFailed { source: ThreadStoreRecordError },
    #[error("Could not connect to the Codex server.")]
    ConnectFailed { source: CodexAppServerConnectError },
    #[error("Could not submit the message.")]
    SendFailed { source: CodexSyncSendError },
}

impl MessageSubmissionSendError {
    fn status(&self) -> StatusCode {
        use MessageSubmissionSendError::*;
        match self {
            FormKeyInvalid {} => StatusCode::FORBIDDEN,
            MessageEmpty {} | OutsideCutoff {} => StatusCode::UNPROCESSABLE_ENTITY,
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
