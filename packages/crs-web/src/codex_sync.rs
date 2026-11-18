use crate::{CodexAppServer, CodexConnection, CodexConnectionRequestError, Config, ThreadHistory, ThreadRecord, ThreadStore, ThreadStoreWriteError};
use codex_app_server_client::{AppServerClient, AppServerEvent};
use codex_app_server_protocol::ThreadSourceKind::*;
use codex_app_server_protocol::{ClientRequest, JSONRPCErrorError, RequestId, ServerNotification, SortDirection, ThreadListParams, ThreadListResponse, ThreadSortKey};
use errgonomic::{ErrorDisplayer, handle, handle_bool};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{Notify, watch};
use tokio::task::JoinSet;
use tokio::time::sleep;
use tracing::warn;
use uuid::Uuid;

pub struct CodexSync {
    pub server_id: String,
    pub source: CodexAppServer,
    pub config: Arc<Config>,
    pub store: ThreadStore,
}

impl CodexSync {
    pub async fn run(self, mut shutdown: watch::Receiver<()>) {
        loop {
            let connection = tokio::select! {
                _ = shutdown.changed() => return,
                result = self.source.connect() => result,
            };
            match connection {
                Ok(mut client) => {
                    let refresh = Notify::new();
                    let connection = CodexConnection {
                        requests: client.request_handle(),
                        reconnect: Notify::new(),
                    };
                    let mut writes = JoinSet::new();
                    let disconnected = tokio::select! {
                        biased;
                        _ = shutdown.changed() => false,
                        () = connection.reconnect.notified() => true,
                        () = self.consume_events(&mut client, &refresh) => true,
                        () = self.reconcile_periodically(&connection, &refresh, &mut writes) => true,
                    };
                    drop(connection);
                    if let Err(error) = client.shutdown().await {
                        warn!(server_id = self.server_id, error = %error, "failed to close Codex connection");
                    }
                    self.finish_writes(&mut writes).await;
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
                        message: "CRS-Web only reads thread history".into(),
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

    async fn reconcile_periodically(&self, client: &CodexConnection, refresh: &Notify, writes: &mut JoinSet<Result<(), ThreadStoreWriteError>>) {
        loop {
            self.reconcile(client, writes).await;
            tokio::select! {
                () = refresh.notified() => (),
                () = sleep(self.config.reconcile_interval()) => (),
            }
        }
    }

    async fn reconcile(&self, client: &CodexConnection, writes: &mut JoinSet<Result<(), ThreadStoreWriteError>>) {
        let mut parents = vec![None];
        let mut discovered = HashSet::new();
        while let Some(parent) = parents.pop() {
            for archived in [false, true] {
                if let Err(error) = self
                    .scan(client, parent.clone(), archived, &mut discovered, &mut parents, writes)
                    .await
                {
                    warn!(server_id = self.server_id, error = %ErrorDisplayer(&error), "failed to list Codex threads");
                }
            }
        }
    }

    async fn scan(&self, client: &CodexConnection, parent: Option<String>, archived: bool, discovered: &mut HashSet<String>, parents: &mut Vec<Option<String>>, writes: &mut JoinSet<Result<(), ThreadStoreWriteError>>) -> Result<(), CodexSyncScanError> {
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
                let history = match ThreadHistory::fetch(client, &thread).await {
                    Ok(history) => history,
                    Err(error) => {
                        warn!(server_id = self.server_id, error = %ErrorDisplayer(&error), "failed to fetch Codex history");
                        continue;
                    }
                };
                let record = ThreadRecord {
                    archived,
                    read_state: read_states.remove(&thread.id),
                    thread,
                };
                let store = self.store.clone();
                let server_id = self.server_id.clone();
                writes.spawn_blocking(move || store.write(&server_id, record, history));
                self.finish_writes(writes).await;
            }
            let Some(next_cursor) = response.next_cursor else {
                return Ok(());
            };
            handle_bool!(!seen_cursors.insert(next_cursor.clone()), CursorInvalid, archived, parent);
            cursor = Some(next_cursor);
        }
    }

    async fn finish_writes(&self, writes: &mut JoinSet<Result<(), ThreadStoreWriteError>>) {
        while let Some(result) = writes.join_next().await {
            match result {
                Ok(Ok(())) => (),
                Ok(Err(error)) => warn!(server_id = self.server_id, error = %ErrorDisplayer(&error), "failed to store Codex history"),
                Err(error) => warn!(server_id = self.server_id, error = %error, "Codex history storage task failed"),
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum CodexSyncScanError {
    #[error("failed to list Codex threads (archived: {archived}, parent: {parent:?})")]
    RequestFailed { source: CodexConnectionRequestError, archived: bool, parent: Option<String> },
    #[error("Codex returned a repeated thread cursor (archived: {archived}, parent: {parent:?})")]
    CursorInvalid { archived: bool, parent: Option<String> },
}
