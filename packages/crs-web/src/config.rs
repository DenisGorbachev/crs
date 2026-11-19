use crate::{CodexAppServer, DEFAULT_CONFIG_RELATIVE_PATH, TranscribeParams};
use errgonomic::{handle, handle_bool, handle_opt, map_err};
use openai_dive::v1::resources::audio::AudioOutputFormat;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env::home_dir;
use std::fs::read_to_string;
use std::io;
use std::num::{NonZeroU16, NonZeroU64};
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;
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

impl TryFrom<&Path> for Config {
    type Error = TryFromPathForConfigError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        use TryFromPathForConfigError::*;
        let text = handle!(read_to_string(path), ReadToStringFailed, path);
        let mut config: Self = handle!(toml::from_str(&text), FromStrFailed, path);
        handle_bool!(!matches!(config.transcribe_params.response_format, None | Some(AudioOutputFormat::Json)), TranscriptionFormatInvalid, path);
        handle_bool!(
            config
                .transcribe_params
                .temperature
                .is_some_and(|value| !(0.0..=1.0).contains(&value)),
            TranscriptionTemperatureInvalid,
            path
        );
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
}

impl TryFrom<PathBuf> for Config {
    type Error = TryFromPathBufForConfigError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        use TryFromPathBufForConfigError::*;
        map_err!(Self::try_from(path.as_path()), TryFromFailed)
    }
}

impl TryFrom<Option<PathBuf>> for Config {
    type Error = TryFromOptionPathBufForConfigError;

    fn try_from(path: Option<PathBuf>) -> Result<Self, Self::Error> {
        use TryFromOptionPathBufForConfigError::*;
        let path = match path {
            Some(path) => path,
            None => handle_opt!(home_dir().filter(|dir| dir.is_absolute()), HomeDirectoryNotFound).join(DEFAULT_CONFIG_RELATIVE_PATH),
        };
        map_err!(Self::try_from(path), TryFromFailed)
    }
}

#[derive(Error, Debug)]
pub enum TryFromPathForConfigError {
    #[error("failed to read CRS-Web configuration at '{}'", path.display())]
    ReadToStringFailed { source: io::Error, path: PathBuf },
    #[error("failed to parse CRS-Web configuration at '{}'", path.display())]
    FromStrFailed { source: toml::de::Error, path: PathBuf },
    #[error("transcribe_params.response_format must be 'json' for gpt-transcribe in '{}'", path.display())]
    TranscriptionFormatInvalid { path: PathBuf },
    #[error("transcribe_params.temperature must be between 0 and 1 in '{}'", path.display())]
    TranscriptionTemperatureInvalid { path: PathBuf },
    #[error("Codex app-server IDs must be nonempty and contain no NUL bytes in '{}'", path.display())]
    ServerIdInvalid { path: PathBuf },
}

#[derive(Error, Debug)]
pub enum TryFromPathBufForConfigError {
    #[error(transparent)]
    TryFromFailed { source: TryFromPathForConfigError },
}

#[derive(Error, Debug)]
pub enum TryFromOptionPathBufForConfigError {
    #[error("could not locate the home directory for CRS-Web. Specify --config <PATH>")]
    HomeDirectoryNotFound {},
    #[error(transparent)]
    TryFromFailed { source: TryFromPathBufForConfigError },
}
