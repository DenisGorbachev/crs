use crate::{Config, ThreadHistory, ThreadPage, ThreadRecord, ThreadSummary};
use errgonomic::{handle, handle_opt};
use fjall::{Database, Keyspace, KeyspaceCreateOptions, PersistMode, Readable};
use std::cmp::Reverse;
use thiserror::Error;

#[derive(Clone)]
pub struct ThreadStore {
    db: Database,
    records: Keyspace,
    histories: Keyspace,
}

impl ThreadStore {
    pub fn open(db: &Database) -> Result<Self, ThreadStoreOpenError> {
        use ThreadStoreOpenError::*;
        let records = handle!(db.keyspace("thread_records", KeyspaceCreateOptions::default), OpenRecordsFailed);
        let histories = handle!(db.keyspace("thread_histories", KeyspaceCreateOptions::default), OpenHistoriesFailed);
        Ok(Self {
            db: db.clone(),
            records,
            histories,
        })
    }

    pub fn write(&self, server_id: &str, record: ThreadRecord, history: ThreadHistory) -> Result<(), ThreadStoreWriteError> {
        use ThreadStoreWriteError::*;
        let thread_id = record.thread.id.as_str();
        let key = format!("{server_id}\0{thread_id}");
        let record = handle!(serde_json::to_vec(&record), EncodeRecordFailed, server_id, thread_id);
        let history = handle!(serde_json::to_vec(&history), EncodeHistoryFailed, server_id, thread_id);
        let old_record = handle!(self.records.get(&key), GetRecordFailed, server_id, thread_id);
        if old_record.as_deref() == Some(record.as_slice()) {
            let old_history = handle!(self.histories.get(&key), GetHistoryFailed, server_id, thread_id);
            if old_history.as_deref() == Some(history.as_slice()) {
                return Ok(());
            }
        }
        let mut batch = self.db.batch().durability(Some(PersistMode::SyncData));
        batch.insert(&self.records, key.as_bytes(), record);
        batch.insert(&self.histories, key, history);
        handle!(batch.commit(), CommitFailed, server_id, thread_id);
        Ok(())
    }

    pub fn read(&self, config: &Config, source: &str, id: &str) -> Result<Option<ThreadPage>, ThreadStoreReadError> {
        use ThreadStoreReadError::*;
        if !config.codex_app_servers.contains_key(source) {
            return Ok(None);
        }
        let key = format!("{source}\0{id}");
        let snapshot = self.db.snapshot();
        let Some(bytes) = handle!(snapshot.get(&self.records, &key), GetRecordFailed) else {
            return Ok(None);
        };
        let record: ThreadRecord = handle!(serde_json::from_slice(&bytes), DecodeRecordFailed);
        if !config.includes_thread(record.thread.created_at) {
            return Ok(None);
        }
        let bytes = handle_opt!(handle!(snapshot.get(&self.histories, &key), GetHistoryFailed), HistoryNotFound);
        let history = handle!(serde_json::from_slice(&bytes), DecodeHistoryFailed);
        Ok(Some(ThreadPage {
            record,
            history,
        }))
    }

    pub fn list<'a>(&self, config: &Config, sources: impl IntoIterator<Item = &'a str>) -> Result<Vec<ThreadSummary>, ThreadStoreListError> {
        use ThreadStoreListError::*;
        sources
            .into_iter()
            .try_fold(Vec::new(), |mut threads, server_id| {
                self.records
                    .prefix(format!("{server_id}\0"))
                    .try_for_each(|entry| {
                        let (_, bytes) = handle!(entry.into_inner(), IntoInnerFailed, server_id);
                        let record: ThreadRecord = handle!(serde_json::from_slice(&bytes), FromSliceFailed, server_id);
                        if config.includes_thread(record.thread.created_at) {
                            threads.push(ThreadSummary::from((server_id, record)));
                        }
                        Ok(())
                    })
                    .map(|()| threads)
            })
            .map(|mut threads| {
                threads.sort_by_key(|thread| Reverse(thread.updated_at));
                threads
            })
    }
}

#[derive(Error, Debug)]
pub enum ThreadStoreOpenError {
    #[error("failed to open the Codex thread records")]
    OpenRecordsFailed { source: fjall::Error },
    #[error("failed to open the Codex thread histories")]
    OpenHistoriesFailed { source: fjall::Error },
}

#[derive(Error, Debug)]
pub enum ThreadStoreWriteError {
    #[error("failed to encode record for Codex thread '{thread_id}' from '{server_id}'")]
    EncodeRecordFailed { source: serde_json::Error, server_id: String, thread_id: String },
    #[error("failed to encode history for Codex thread '{thread_id}' from '{server_id}'")]
    EncodeHistoryFailed { source: serde_json::Error, server_id: String, thread_id: String },
    #[error("failed to read record for Codex thread '{thread_id}' from '{server_id}'")]
    GetRecordFailed { source: fjall::Error, server_id: String, thread_id: String },
    #[error("failed to read history for Codex thread '{thread_id}' from '{server_id}'")]
    GetHistoryFailed { source: fjall::Error, server_id: String, thread_id: String },
    #[error("failed to save Codex thread '{thread_id}' from '{server_id}'")]
    CommitFailed { source: fjall::Error, server_id: String, thread_id: String },
}

#[derive(Error, Debug)]
pub enum ThreadStoreListError {
    #[error("failed to read a saved Codex thread from '{server_id}'")]
    IntoInnerFailed { source: fjall::Error, server_id: String },
    #[error("failed to decode a saved Codex thread from '{server_id}'")]
    FromSliceFailed { source: serde_json::Error, server_id: String },
}

#[derive(Error, Debug)]
pub enum ThreadStoreReadError {
    #[error("failed to read the saved thread record")]
    GetRecordFailed { source: fjall::Error },
    #[error("failed to decode the saved thread record")]
    DecodeRecordFailed { source: serde_json::Error },
    #[error("failed to read the saved thread history")]
    GetHistoryFailed { source: fjall::Error },
    #[error("saved thread record has no history")]
    HistoryNotFound {},
    #[error("failed to decode the saved thread history")]
    DecodeHistoryFailed { source: serde_json::Error },
}
