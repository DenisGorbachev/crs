use crate::{CodexSync, Config, ConfigLoadError, LoadFormKeyError, ThreadStore, ThreadStoreOpenError, load_form_key};
use derive_more::{Deref, DerefMut};
use errgonomic::handle;
use fjall::Database;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Deref, DerefMut)]
pub struct Server {
    pub config: Arc<Config>,
    pub sources: BTreeMap<String, Arc<CodexSync>>,
    pub form_key: Uuid,
    pub threads: ThreadStore,
    #[deref]
    #[deref_mut]
    pub db: Database,
}

impl Server {
    pub fn open(path: PathBuf) -> Result<Self, ServerOpenError> {
        use ServerOpenError::*;
        let config = handle!(Config::load(path), LoadFailed);
        let db = handle!(Database::builder(&config.db_path).open(), OpenFailed, path: config.db_path);
        let form_key = handle!(load_form_key(&db, config.form_key_env.as_deref()), LoadFormKeyFailed);
        let threads = handle!(ThreadStore::open(&db), OpenThreadsFailed);
        let config = Arc::new(config);
        let sources = config
            .codex_app_servers
            .iter()
            .map(|(id, source)| {
                (
                    id.clone(),
                    Arc::new(CodexSync {
                        config: config.clone(),
                        server_id: id.clone(),
                        source: source.clone(),
                        store: threads.clone(),
                        connection: Default::default(),
                        writers: Default::default(),
                        refresh: Default::default(),
                        viewed: Default::default(),
                        viewed_changed: Default::default(),
                    }),
                )
            })
            .collect();
        Ok(Self {
            config,
            sources,
            form_key,
            threads,
            db,
        })
    }
}

#[derive(Error, Debug)]
pub enum ServerOpenError {
    #[error("failed to initialize the form key")]
    LoadFormKeyFailed { source: LoadFormKeyError },
    #[error("failed to open CRS-Web thread storage")]
    OpenThreadsFailed { source: ThreadStoreOpenError },
    #[error("failed to load CRS-Web configuration")]
    LoadFailed { source: ConfigLoadError },
    #[error("failed to open Fjall database at '{}'", path.display())]
    OpenFailed { source: fjall::Error, path: PathBuf },
}
