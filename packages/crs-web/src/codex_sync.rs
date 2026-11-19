use crate::{CodexAppServer, CodexConnection, CodexConnectionRequestError, Config, ThreadHistory, ThreadHistoryFetchError, ThreadRecord, ThreadRecordRefreshMetadataError, ThreadStore, ThreadStoreRecordError, ThreadStoreWriteError};
use codex_app_server_client::{AppServerClient, AppServerEvent};
use codex_app_server_protocol::ThreadSourceKind::*;
use codex_app_server_protocol::{ClientRequest, JSONRPCErrorError, RequestId, ServerNotification, SortDirection, ThreadListParams, ThreadListResponse, ThreadSortKey, ThreadStatus};
use errgonomic::{ErrorDisplayer, handle, handle_bool};
use futures::{StreamExt, stream};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Weak};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{Mutex, Notify, OwnedMutexGuard, watch};
use tokio::task::{JoinError, spawn_blocking};
use tokio::time::{MissedTickBehavior, interval, sleep};
use tracing::warn;
use uuid::Uuid;

pub struct CodexSync {
    pub server_id: String,
    pub source: CodexAppServer,
    pub config: Arc<Config>,
    pub store: ThreadStore,
    pub writers: Mutex<HashMap<String, Weak<Mutex<()>>>>,
    pub refresh: Notify,
    pub viewed_changed: Notify,
    pub viewed: Arc<Mutex<HashMap<String, watch::Sender<()>>>>,
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
                        () = self.consume_events(&mut client) => true,
                        () = self.refresh_viewed_periodically(&connection) => true,
                        () = self.reconcile_periodically(&connection) => true,
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

    async fn consume_events(&self, client: &mut AppServerClient) {
        while let Some(event) = client.next_event().await {
            match event {
                AppServerEvent::ServerNotification(notification) => {
                    if let ServerNotification::ThreadStatusChanged(event) = *notification {
                        if self.is_viewed(&event.thread_id).await {
                            self.viewed_changed.notify_one();
                        }
                        self.refresh.notify_one();
                    }
                }
                AppServerEvent::Lagged {
                    skipped,
                } => {
                    warn!(server_id = self.server_id, skipped, "Codex events were missed");
                    self.refresh.notify_one();
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

    async fn reconcile_periodically(&self, client: &CodexConnection) {
        loop {
            self.reconcile(client).await;
            tokio::select! {
                () = self.refresh.notified() => (),
                () = sleep(self.config.reconcile_interval()) => (),
            }
        }
    }

    async fn is_viewed(&self, id: &str) -> bool {
        self.viewed
            .lock()
            .await
            .get(id)
            .is_some_and(|sender| sender.receiver_count() != 0)
    }

    async fn lock_thread(&self, id: &str) -> OwnedMutexGuard<()> {
        let writer = {
            let mut writers = self.writers.lock().await;
            writers.retain(|_, writer| writer.strong_count() != 0);
            if let Some(writer) = writers.get(id).and_then(Weak::upgrade) {
                writer
            } else {
                let writer = Arc::new(Mutex::new(()));
                writers.insert(id.to_owned(), Arc::downgrade(&writer));
                writer
            }
        };
        writer.lock_owned().await
    }

    async fn refresh_viewed_periodically(&self, client: &CodexConnection) {
        let mut tick = interval(self.config.thread_refresh_interval());
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = tick.tick() => (),
                () = self.viewed_changed.notified() => (),
            }
            let ids: Vec<_> = {
                let mut viewed = self.viewed.lock().await;
                viewed.retain(|_, sender| sender.receiver_count() != 0);
                viewed.keys().cloned().collect()
            };
            stream::iter(ids)
                .for_each_concurrent(4, |id| async move {
                    if let Err(error) = self.refresh_viewed(client, &id).await {
                        warn!(server_id = self.server_id, %id, error = %ErrorDisplayer(&error), "failed to refresh viewed thread");
                    }
                })
                .await;
        }
    }

    async fn refresh_viewed(&self, client: &CodexConnection, id: &str) -> Result<(), CodexSyncRefreshViewedError> {
        use CodexSyncRefreshViewedError::*;
        let guard = self.lock_thread(id).await;
        let store = self.store.clone();
        let config = self.config.clone();
        let source = self.server_id.clone();
        let thread_id = id.to_owned();
        let result = spawn_blocking(move || store.record(&store.snapshot(), &config, &source, &thread_id)).await;
        let Some(mut record) = handle!(handle!(result, JoinFailed), RecordFailed) else {
            return Ok(());
        };
        if handle!(record.refresh_metadata(client).await, RefreshMetadataFailed) || matches!(record.thread.status, ThreadStatus::Active { .. }) {
            handle!(self.save_thread(client, record, guard).await, SaveThreadFailed);
        }
        Ok(())
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
                let mut record = ThreadRecord {
                    archived,
                    last_turn_id: None,
                    turn_versions: None,
                    read_state: read_states.remove(&thread.id),
                    thread,
                };
                let guard = self.lock_thread(&record.thread.id).await;
                if self.is_viewed(&record.thread.id).await
                    && let Err(error) = record.refresh_metadata(client).await
                {
                    warn!(server_id = self.server_id, error = %ErrorDisplayer(&error), "failed to refresh listed thread metadata");
                    continue;
                }
                if let Err(error) = self.save_thread(client, record, guard).await {
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

    async fn save_thread(&self, client: &CodexConnection, record: ThreadRecord, guard: OwnedMutexGuard<()>) -> Result<(), CodexSyncSaveThreadError> {
        use CodexSyncSaveThreadError::*;
        let history = handle!(ThreadHistory::fetch(client, &record.thread).await, FetchFailed);
        let id = record.thread.id.clone();
        let viewed = self.viewed.clone();
        let store = self.store.clone();
        let server_id = self.server_id.clone();
        let write = spawn_blocking(move || {
            let _guard = guard;
            store.write(&server_id, record, history).map(|changed| {
                if changed && let Some(sender) = viewed.blocking_lock().get(&id) {
                    sender.send_replace(());
                }
            })
        })
        .await;
        handle!(handle!(write, JoinFailed), WriteFailed);
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum CodexSyncRefreshViewedError {
    #[error("thread record task failed")]
    JoinFailed { source: JoinError },
    #[error("failed to read the saved thread record")]
    RecordFailed { source: ThreadStoreRecordError },
    #[error("failed to read current Codex thread metadata")]
    RefreshMetadataFailed { source: ThreadRecordRefreshMetadataError },
    #[error("failed to save the viewed thread")]
    SaveThreadFailed { source: CodexSyncSaveThreadError },
}

#[derive(Error, Debug)]
pub enum CodexSyncSaveThreadError {
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
