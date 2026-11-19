use crate::{CodexAppServer, Config};
use errgonomic::{handle, handle_bool, handle_opt};
use openai_dive::v1::resources::audio::AudioOutputFormat;
use std::env::home_dir;
use std::fs::read_to_string;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

pub fn load_config(path: Option<PathBuf>) -> Result<Config, LoadConfigError> {
    use LoadConfigError::*;
    let path = match path {
        Some(path) => path,
        None => handle_opt!(home_dir().filter(|dir| dir.is_absolute()), HomeDirectoryNotFound).join(".config/crs/web.toml"),
    };
    let text = handle!(read_to_string(&path), ReadToStringFailed, path);
    let mut config: Config = handle!(toml::from_str(&text), FromStrFailed, path);
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

#[derive(Error, Debug)]
pub enum LoadConfigError {
    #[error("could not locate the home directory for CRS-Web. Specify --config <PATH>")]
    HomeDirectoryNotFound {},
    #[error("transcribe_params.response_format must be 'json' for gpt-transcribe in '{}'", path.display())]
    TranscriptionFormatInvalid { path: PathBuf },
    #[error("transcribe_params.temperature must be between 0 and 1 in '{}'", path.display())]
    TranscriptionTemperatureInvalid { path: PathBuf },
    #[error("failed to read CRS-Web configuration at '{}'", path.display())]
    ReadToStringFailed { source: io::Error, path: PathBuf },
    #[error("failed to parse CRS-Web configuration at '{}'", path.display())]
    FromStrFailed { source: toml::de::Error, path: PathBuf },
    #[error("Codex app-server IDs must be nonempty and contain no NUL bytes in '{}'", path.display())]
    ServerIdInvalid { path: PathBuf },
}
