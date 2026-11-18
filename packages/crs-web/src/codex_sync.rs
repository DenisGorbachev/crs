use crate::{CodexAppServer, CodexConnection, CodexConnectionRequestError, Config, ThreadHistory, ThreadHistoryFetchError, ThreadRecord, ThreadStore, ThreadStoreWriteError};
use codex_app_server_client::{AppServerClient, AppServerEvent};
use codex_app_server_protocol::ThreadSourceKind::*;
use codex_app_server_protocol::{ClientRequest, JSONRPCErrorError, RequestId, ServerNotification, SortDirection, ThreadListParams, ThreadListResponse, ThreadSortKey};
use errgonomic::{ErrorDisplayer, handle, handle_bool};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{Mutex, Notify, watch};
use tokio::task::{JoinError, spawn_blocking};
use tokio::time::sleep;
use tracing::warn;
use uuid::Uuid;

pub struct CodexSync {
    pub server_id: String,
    pub source: CodexAppServer,
    pub config: Arc<Config>,
    pub store: ThreadStore,
    pub writer: Arc<Mutex<()>>,
    pub refresh: Notify,
}

impl CodexSync {
    pub async fn run(self: Arc<Self>, mut shutdown: watch::Receiver<()>) {
        loop {
            let connection = tokio::select! {
                _ = shutdown.changed() => return,
                result = self.source.connect() => result,
            };
            match connection {
                Ok(mut client) => {
                    let connection = CodexConnection::from(&client);
                    let disconnected = tokio::select! {
                        biased;
                        _ = shutdown.changed() => false,
                        () = connection.reconnect.notified() => true,
                        () = self.consume_events(&mut client, &self.refresh) => true,
                        () = self.reconcile_periodically(&connection, &self.refresh) => true,
                    };
                    drop(connection);
                    if let Err(error) = client.shutdown().await {
                        warn!(server_id = self.server_id, error = %error, "failed to close Codex connection");
                    }
                    if !disconnected {
                        return;
                    }
                }
                Err(error) => warn!(server_id = self.server_id, error = %ErrorDisplayer(&error), "Codex connection failed"),
            }
            tokio::select! {
                _ = shutdown.changed() => return,
                () = sleep(Duration::from_secs(5)) => (),
            }
        }
    }

    async fn consume_events(&self, client: &mut AppServerClient, refresh: &Notify) {
        while let Some(event) = client.next_event().await {
            match event {
                AppServerEvent::ServerNotification(notification) if matches!(*notification, ServerNotification::ThreadStatusChanged(_)) => refresh.notify_one(),
                AppServerEvent::Lagged {
                    skipped,
                } => {
                    warn!(server_id = self.server_id, skipped, "Codex events were missed");
                    refresh.notify_one();
                }
                AppServerEvent::Disconnected {
                    message,
                } => {
                    warn!(server_id = self.server_id, message, "Codex connection lost");
                    return;
                }
                AppServerEvent::ServerRequest(request) => {
                    let error = JSONRPCErrorError {
                        code: -32601,
                        data: None,
                        message: "This connection synchronizes thread history".into(),
                    };
                    if let Err(error) = client
                        .reject_server_request(request.id().clone(), error)
                        .await
                    {
                        warn!(server_id = self.server_id, error = %error, "failed to reject Codex server request");
                    }
                }
                _ => (),
            }
        }
    }

    async fn reconcile_periodically(&self, client: &CodexConnection, refresh: &Notify) {
        loop {
            self.reconcile(client).await;
            tokio::select! {
                () = refresh.notified() => (),
                () = sleep(self.config.reconcile_interval()) => (),
            }
        }
    }

    async fn reconcile(&self, client: &CodexConnection) {
        let mut parents = vec![None];
        let mut discovered = HashSet::new();
        while let Some(parent) = parents.pop() {
            for archived in [false, true] {
                if let Err(error) = self
                    .scan(client, parent.clone(), archived, &mut discovered, &mut parents)
                    .await
                {
                    warn!(server_id = self.server_id, error = %ErrorDisplayer(&error), "failed to list Codex threads");
                }
            }
        }
    }

    async fn scan(&self, client: &CodexConnection, parent: Option<String>, archived: bool, discovered: &mut HashSet<String>, parents: &mut Vec<Option<String>>) -> Result<(), CodexSyncScanError> {
        use CodexSyncScanError::*;
        let mut seen_cursors = HashSet::new();
        let mut cursor = None;
        loop {
            let response: ThreadListResponse = handle!(
                client
                    .request(ClientRequest::ThreadList {
                        request_id: RequestId::String(Uuid::new_v4().to_string()),
                        params: ThreadListParams {
                            include_attached_threads: false,
                            current_account_only: false,
                            cursor,
                            limit: None,
                            excluded_thread_ids: None,
                            sort_key: Some(ThreadSortKey::CreatedAt),
                            sort_direction: Some(SortDirection::Desc),
                            model_providers: Some(Vec::new()),
                            source_kinds: parent
                                .is_none()
                                .then(|| vec![Cli, VsCode, Exec, AppServer, SubAgent, Unknown]),
                            originators: None,
                            archived: Some(archived),
                            section_id: None,
                            project_id: None,
                            cwd: None,
                            use_state_db_only: false,
                            search_term: None,
                            parent_thread_id: None,
                            ancestor_thread_id: None,
                            attached_to_thread_id: parent.clone(),
                        },
                    })
                    .await,
                RequestFailed,
                archived,
                parent
            );
            let mut read_states = response.read_states.unwrap_or_default();
            for thread in response.data {
                if thread.ephemeral || !discovered.insert(thread.id.clone()) {
                    continue;
                }
                parents.push(Some(thread.id.clone()));
                if !self.config.includes_thread(thread.created_at) {
                    continue;
                }
                let record = ThreadRecord {
                    archived,
                    last_turn_id: None,
                    read_state: read_states.remove(&thread.id),
                    thread,
                };
                if let Err(error) = self.save_thread(client, record, None).await {
                    warn!(server_id = self.server_id, error = %ErrorDisplayer(&error), "failed to synchronize Codex thread");
                }
            }
            let Some(next_cursor) = response.next_cursor else {
                return Ok(());
            };
            handle_bool!(!seen_cursors.insert(next_cursor.clone()), CursorInvalid, archived, parent);
            cursor = Some(next_cursor);
        }
    }

    pub async fn save_thread(&self, client: &CodexConnection, record: ThreadRecord, expected_message: Option<(&str, &str)>) -> Result<(), CodexSyncSaveThreadError> {
        use CodexSyncSaveThreadError::*;
        let guard = self.writer.clone().lock_owned().await;
        let mut fetched = ThreadHistory::fetch(client, &record.thread).await;
        if let Some((turn_id, message_id)) = expected_message {
            for delay in [75, 150, 300, 600, 1000] {
                if fetched
                    .as_ref()
                    .is_ok_and(|history| history.contains_message(turn_id, message_id))
                {
                    break;
                }
                sleep(Duration::from_millis(delay)).await;
                fetched = ThreadHistory::fetch(client, &record.thread).await;
            }
            handle_bool!(
                fetched
                    .as_ref()
                    .is_ok_and(|history| !history.contains_message(turn_id, message_id)),
                MessageNotFound,
                turn_id,
                message_id
            );
        }
        let history = handle!(fetched, FetchFailed);
        let store = self.store.clone();
        let server_id = self.server_id.clone();
        let write = spawn_blocking(move || {
            let _guard = guard;
            store.write(&server_id, record, history)
        })
        .await;
        handle!(handle!(write, JoinFailed), WriteFailed);
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum CodexSyncSaveThreadError {
    #[error("acknowledged message '{message_id}' is not visible in turn '{turn_id}' yet")]
    MessageNotFound { turn_id: String, message_id: String },
    #[error("failed to fetch Codex thread history")]
    FetchFailed { source: ThreadHistoryFetchError },
    #[error("thread storage task failed")]
    JoinFailed { source: JoinError },
    #[error("failed to store Codex thread history")]
    WriteFailed { source: ThreadStoreWriteError },
}

#[derive(Error, Debug)]
pub enum CodexSyncScanError {
    #[error("failed to list Codex threads (archived: {archived}, parent: {parent:?})")]
    RequestFailed { source: CodexConnectionRequestError, archived: bool, parent: Option<String> },
    #[error("Codex returned a repeated thread cursor (archived: {archived}, parent: {parent:?})")]
    CursorInvalid { archived: bool, parent: Option<String> },
}

mod send;
pub use send::*;
