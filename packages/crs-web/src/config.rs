use crate::{CodexAppServer, TranscribeParams};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::num::{NonZeroU16, NonZeroU64};
use std::path::PathBuf;
use std::time::Duration;
use time::OffsetDateTime;

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default, with = "time::serde::rfc3339::option", skip_serializing_if = "Option::is_none")]
    pub threads_created_since: Option<OffsetDateTime>,
    pub db_path: PathBuf,
    #[serde(default)]
    pub transcribe_params: TranscribeParams,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form_key_env: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconcile_interval_seconds: Option<NonZeroU64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_refresh_interval_seconds: Option<NonZeroU16>,
    pub codex_app_servers: BTreeMap<String, CodexAppServer>,
}

impl Config {
    pub fn includes_thread(&self, created_at: i64) -> bool {
        self.threads_created_since
            .is_none_or(|since| OffsetDateTime::from_unix_timestamp(created_at).is_ok_and(|created| created >= since))
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
