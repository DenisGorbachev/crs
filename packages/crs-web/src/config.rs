use crate::CodexAppServer;
use errgonomic::{handle, handle_bool};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::read_to_string;
use std::io;
use std::num::{NonZeroU16, NonZeroU64};
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;
use time::OffsetDateTime;

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(with = "time::serde::rfc3339")]
    pub threads_created_since: OffsetDateTime,
    pub db_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconcile_interval_seconds: Option<NonZeroU64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_refresh_interval_seconds: Option<NonZeroU16>,
    pub codex_app_servers: BTreeMap<String, CodexAppServer>,
}

impl Config {
    pub fn load(path: PathBuf) -> Result<Self, ConfigLoadError> {
        use ConfigLoadError::*;
        let text = handle!(read_to_string(&path), ReadToStringFailed, path);
        let mut config: Self = handle!(toml::from_str(&text), FromStrFailed, path);
        handle_bool!(
            config
                .codex_app_servers
                .keys()
                .any(|id| id.is_empty() || id.contains('\0')),
            ServerIdInvalid,
            path
        );
        if let Some(dir) = path.parent() {
            config.db_path = dir.join(config.db_path);
            config.codex_app_servers.values_mut().for_each(|server| {
                if let CodexAppServer::UnixSocket {
                    socket_path: Some(socket_path),
                } = server
                {
                    *socket_path = dir.join(&*socket_path);
                }
            });
        }
        Ok(config)
    }

    pub fn includes_thread(&self, created_at: i64) -> bool {
        OffsetDateTime::from_unix_timestamp(created_at).is_ok_and(|created_at| created_at >= self.threads_created_since)
    }

    pub fn thread_refresh_interval(&self) -> Duration {
        Duration::from_secs(
            self.thread_refresh_interval_seconds
                .map_or(3, NonZeroU16::get)
                .into(),
        )
    }

    pub fn reconcile_interval(&self) -> Duration {
        Duration::from_secs(self.reconcile_interval_seconds.map_or(60, NonZeroU64::get))
    }
}

#[derive(Error, Debug)]
pub enum ConfigLoadError {
    #[error("failed to read CRS-Web configuration at '{}'", path.display())]
    ReadToStringFailed { source: io::Error, path: PathBuf },
    #[error("failed to parse CRS-Web configuration at '{}'", path.display())]
    FromStrFailed { source: toml::de::Error, path: PathBuf },
    #[error("Codex app-server IDs must be nonempty and contain no NUL bytes in '{}'", path.display())]
    ServerIdInvalid { path: PathBuf },
}
