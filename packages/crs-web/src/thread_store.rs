use crate::{Config, ThreadHistory, ThreadHistoryIntoVisibleTurnsError, ThreadPage, ThreadRecord, ThreadSummary, canonical_json, turn_versions};
use errgonomic::{handle, handle_opt};
use fjall::{Database, Keyspace, KeyspaceCreateOptions, PersistMode, Readable, Snapshot};
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

    pub fn write(&self, server_id: &str, mut record: ThreadRecord, history: ThreadHistory) -> Result<bool, ThreadStoreWriteError> {
        use ThreadStoreWriteError::*;
        record.last_turn_id = history.last_turn_id().map(str::to_owned);
        let thread_id = record.thread.id.as_str();
        let bytes = handle!(canonical_json(&history).and_then(|json| serde_json::to_vec(&json)), EncodeHistoryFailed, server_id, thread_id);
        let turns = handle!(history.into_visible_turns(), IntoVisibleTurnsFailed, server_id, thread_id);
        record.turn_versions = Some(handle!(turn_versions(&turns), EncodeVersionsFailed, server_id, thread_id));
        let key = format!("{server_id}\0{thread_id}");
        let record = handle!(canonical_json(&record).and_then(|json| serde_json::to_vec(&json)), EncodeRecordFailed, server_id, thread_id);
        let old_record = handle!(self.records.get(&key), GetRecordFailed, server_id, thread_id);
        let changed = old_record.as_deref() != Some(record.as_slice());
        if !changed {
            let old_history = handle!(self.histories.get(&key), GetHistoryFailed, server_id, thread_id);
            if old_history.as_deref() == Some(bytes.as_slice()) {
                return Ok(false);
            }
        }
        let mut batch = self.db.batch().durability(Some(PersistMode::SyncData));
        batch.insert(&self.records, key.as_bytes(), record);
        batch.insert(&self.histories, key, bytes);
        handle!(batch.commit(), CommitFailed, server_id, thread_id);
        Ok(changed)
    }

    pub fn record(&self, snapshot: &Snapshot, config: &Config, source: &str, id: &str) -> Result<Option<ThreadRecord>, ThreadStoreRecordError> {
        use ThreadStoreRecordError::*;
        if !config.codex_app_servers.contains_key(source) {
            return Ok(None);
        }
        let key = format!("{source}\0{id}");
        let Some(bytes) = handle!(snapshot.get(&self.records, &key), GetRecordFailed) else {
            return Ok(None);
        };
        let record: ThreadRecord = handle!(serde_json::from_slice(&bytes), DecodeRecordFailed);
        Ok(config
            .includes_thread(record.thread.created_at)
            .then_some(record))
    }

    pub fn read(&self, config: &Config, source: &str, id: &str) -> Result<Option<ThreadPage>, ThreadStoreReadError> {
        use ThreadStoreReadError::*;
        let snapshot = self.db.snapshot();
        let Some(record) = handle!(self.record(&snapshot, config, source, id), RecordFailed) else {
            return Ok(None);
        };
        Ok(Some(handle!(self.page(&snapshot, source, record), PageFailed)))
    }

    pub fn snapshot(&self) -> Snapshot {
        self.db.snapshot()
    }

    pub fn page(&self, snapshot: &Snapshot, source: &str, mut record: ThreadRecord) -> Result<ThreadPage, ThreadStorePageError> {
        use ThreadStorePageError::*;
        let id = &record.thread.id;
        let key = format!("{source}\0{id}");
        let bytes = handle_opt!(handle!(snapshot.get(&self.histories, &key), GetHistoryFailed), HistoryNotFound);
        let history: ThreadHistory = handle!(serde_json::from_slice(&bytes), DecodeHistoryFailed);
        let turns = handle!(history.into_visible_turns(), IntoVisibleTurnsFailed);
        let versions = match record.turn_versions.take() {
            Some(versions) => versions,
            None => handle!(turn_versions(&turns), EncodeVersionsFailed),
        };
        Ok(ThreadPage {
            versions,
            record,
            turns,
        })
    }

    pub fn list<'a>(&self, config: &Config, sources: impl IntoIterator<Item = &'a str>) -> Result<Vec<ThreadSummary>, ThreadStoreListError> {
        use ThreadStoreListError::*;
        let snapshot = self.db.snapshot();
        sources
            .into_iter()
            .try_fold(Vec::new(), |mut threads, server_id| {
                snapshot
                    .prefix(&self.records, format!("{server_id}\0"))
                    .try_for_each(|entry| {
                        let (key, bytes) = handle!(entry.into_inner(), IntoInnerFailed, server_id);
                        let mut record: ThreadRecord = handle!(serde_json::from_slice(&bytes), FromSliceFailed, server_id);
                        if record.is_parent() && config.includes_thread(record.thread.created_at) {
                            if record.last_turn_id.is_none() {
                                let bytes = handle_opt!(handle!(snapshot.get(&self.histories, key), GetHistoryFailed, server_id), HistoryNotFound, server_id);
                                let history: ThreadHistory = handle!(serde_json::from_slice(&bytes), DecodeHistoryFailed, server_id);
                                record.last_turn_id = history.last_turn_id().map(str::to_owned);
                            }
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
    #[error("failed to resolve turns for Codex thread '{thread_id}' from '{server_id}'")]
    IntoVisibleTurnsFailed { source: ThreadHistoryIntoVisibleTurnsError, server_id: String, thread_id: String },
    #[error("failed to fingerprint turns for Codex thread '{thread_id}' from '{server_id}'")]
    EncodeVersionsFailed { source: serde_json::Error, server_id: String, thread_id: String },
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
    #[error("failed to read saved history from '{server_id}'")]
    GetHistoryFailed { source: fjall::Error, server_id: String },
    #[error("missing saved history from '{server_id}'")]
    HistoryNotFound { server_id: String },
    #[error("failed to decode saved history from '{server_id}'")]
    DecodeHistoryFailed { source: serde_json::Error, server_id: String },
    #[error("failed to read a saved Codex thread from '{server_id}'")]
    IntoInnerFailed { source: fjall::Error, server_id: String },
    #[error("failed to decode a saved Codex thread from '{server_id}'")]
    FromSliceFailed { source: serde_json::Error, server_id: String },
}

#[derive(Error, Debug)]
pub enum ThreadStoreRecordError {
    #[error("failed to read the saved thread record")]
    GetRecordFailed { source: fjall::Error },
    #[error("failed to decode the saved thread record")]
    DecodeRecordFailed { source: serde_json::Error },
}

#[derive(Error, Debug)]
pub enum ThreadStoreReadError {
    #[error("failed to read the saved thread record")]
    RecordFailed { source: ThreadStoreRecordError },
    #[error("failed to load the saved thread page")]
    PageFailed { source: ThreadStorePageError },
}

#[derive(Error, Debug)]
pub enum ThreadStorePageError {
    #[error("failed to resolve saved thread turns")]
    IntoVisibleTurnsFailed { source: ThreadHistoryIntoVisibleTurnsError },
    #[error("failed to fingerprint saved turns")]
    EncodeVersionsFailed { source: serde_json::Error },
    #[error("failed to read the saved thread history")]
    GetHistoryFailed { source: fjall::Error },
    #[error("saved thread record has no history")]
    HistoryNotFound {},
    #[error("failed to decode the saved thread history")]
    DecodeHistoryFailed { source: serde_json::Error },
}
