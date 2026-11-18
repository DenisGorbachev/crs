use crate::{Config, ConfigLoadError};
use derive_more::{Deref, DerefMut};
use errgonomic::handle;
use fjall::Database;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

#[derive(Deref, DerefMut)]
pub struct Server {
    pub config: Arc<Config>,
    #[deref]
    #[deref_mut]
    pub db: Database,
}

impl Server {
    pub fn open(path: PathBuf) -> Result<Self, ServerOpenError> {
        use ServerOpenError::*;
        let config = handle!(Config::load(path), LoadFailed);
        let db = handle!(Database::builder(&config.db_path).open(), OpenFailed, path: config.db_path);
        Ok(Self {
            config: Arc::new(config),
            db,
        })
    }
}

#[derive(Error, Debug)]
pub enum ServerOpenError {
    #[error("failed to load CRS-Web configuration")]
    LoadFailed { source: ConfigLoadError },
    #[error("failed to open Fjall database at '{}'", path.display())]
    OpenFailed { source: fjall::Error, path: PathBuf },
}
