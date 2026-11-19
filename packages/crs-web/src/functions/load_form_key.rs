use errgonomic::{handle, handle_discard, handle_opt};
use fjall::{Database, KeyspaceCreateOptions, PersistMode};
use std::env::var_os;
use thiserror::Error;
use uuid::Uuid;

pub fn load_form_key(db: &Database, env_name: Option<&str>) -> Result<Uuid, LoadFormKeyError> {
    use LoadFormKeyError::*;
    if let Some(name) = env_name {
        let value = handle_opt!(var_os(name), EnvNotFound, name);
        let text = handle_opt!(value.to_str(), EnvInvalid, name);
        return Ok(handle_discard!(Uuid::parse_str(text), EnvInvalid, name));
    }
    let settings = handle!(db.keyspace("server_settings", KeyspaceCreateOptions::default), OpenSettingsFailed);
    let key = "form_key";
    if let Some(bytes) = handle!(settings.get(key), ReadKeyFailed) {
        return Ok(handle!(Uuid::from_slice(&bytes), DecodeKeyFailed));
    }
    let value = Uuid::new_v4();
    let mut batch = db.batch().durability(Some(PersistMode::SyncAll));
    batch.insert(&settings, key, value.as_bytes().as_slice());
    handle!(batch.commit(), SaveKeyFailed);
    Ok(value)
}

#[derive(Debug, Error)]
pub enum LoadFormKeyError {
    #[error("form key environment variable '{name}' is missing")]
    EnvNotFound { name: String },
    #[error("expected a UUID in form key environment variable '{name}'")]
    EnvInvalid { name: String },
    #[error("failed to open CRS-Web server settings")]
    OpenSettingsFailed { source: fjall::Error },
    #[error("failed to read the saved form key")]
    ReadKeyFailed { source: fjall::Error },
    #[error("the saved form key is invalid")]
    DecodeKeyFailed { source: uuid::Error },
    #[error("failed to persist the generated form key")]
    SaveKeyFailed { source: fjall::Error },
}
