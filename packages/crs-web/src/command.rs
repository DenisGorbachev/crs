use crate::{App, Config, DEFAULT_CONFIG_RELATIVE_PATH, MAX_RECORDING_BYTES, Server, ServerOpenError, SharedServer, TryFromOptionPathBufForConfigError, open_file, submit_message, thread_events, thread_updates, transcribe};
use axum::extract::DefaultBodyLimit;
use axum::http::header::CONTENT_TYPE;
use axum::routing::{get, post};
use axum::{Extension, Router, serve};
use clap::Parser;
use dioxus::cli_config::fullstack_address_or_localhost;
use dioxus::server::{DioxusRouterExt, ServeConfig};
use errgonomic::handle;
use openai_dive::v1::api::Client;
use secrecy::{ExposeSecret, SecretString};
use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::signal::ctrl_c;
use tokio::sync::{Mutex, watch};
use tokio::task::{JoinError, JoinSet};
use tracing::{info, warn};

#[derive(Parser)]
#[command(author, version, about, propagate_version = true, flatten_help = true, disable_help_subcommand = true)]
pub struct Command {
    #[arg(long, value_name = "PATH", help = format!("Config file (defaults to ~/{DEFAULT_CONFIG_RELATIVE_PATH})"))]
    config: Option<PathBuf>,
    #[arg(long, env = "OPENAI_API_KEY", hide_env_values = true)]
    openai_api_key: Option<SecretString>,
}

impl Command {
    pub async fn run(self) -> Result<ExitCode, CommandRunError> {
        use CommandRunError::*;
        let config = handle!(Config::try_from(self.config), TryFromFailed);
        let server = handle!(Server::open(config), OpenServerFailed);
        let store = server.threads.clone();
        let config = server.config.clone();
        let sources: Vec<_> = server.sources.values().cloned().collect();
        let form_key = server.form_key;
        let openai = self
            .openai_api_key
            .filter(|key| !key.expose_secret().trim().is_empty())
            .map(|key| Arc::new(Client::new(key.expose_secret().to_owned())));
        let server: SharedServer = Arc::new(Mutex::new(server));
        let (shutdown, receiver) = watch::channel(());
        let router = Router::new()
            .route("/api/transcribe", post(transcribe).layer(DefaultBodyLimit::max(MAX_RECORDING_BYTES)))
            .layer(Extension(openai.clone()))
            .route("/api/thread-events", get(thread_events))
            .route("/api/thread-updates", post(thread_updates))
            .route("/messages", post(submit_message))
            .route("/servers/{source}/files/{*path}", get(open_file))
            .route("/voice.js", get(async || ([(CONTENT_TYPE, "text/javascript; charset=utf-8")], include_str!("voice.js"))))
            .route("/voice-pcm.js", get(async || ([(CONTENT_TYPE, "text/javascript; charset=utf-8")], include_str!("voice_pcm.js"))))
            .route("/thread-queue.js", get(async || ([(CONTENT_TYPE, "text/javascript; charset=utf-8")], include_str!("thread_queue.js"))))
            .route("/thread-review.js", get(async || ([(CONTENT_TYPE, "text/javascript; charset=utf-8")], include_str!("thread_review.js"))))
            .serve_api_application(
                ServeConfig::new()
                    .context(form_key)
                    .context(openai.is_some())
                    .context(config)
                    .context(store.clone()),
                App,
            )
            .layer(Extension(server))
            .layer(Extension(receiver.clone()));
        let address = fullstack_address_or_localhost();
        let listener = handle!(TcpListener::bind(address).await, BindFailed, address);
        let mut workers = JoinSet::new();
        sources.into_iter().for_each(|source| {
            workers.spawn(source.run(receiver.clone()));
        });
        info!(%address, "serving CRS-Web");
        let result = serve(listener, router)
            .with_graceful_shutdown(async move {
                if let Err(error) = ctrl_c().await {
                    warn!(%error, "failed to listen for Ctrl+C");
                }
                drop(shutdown);
            })
            .await;
        while let Some(result) = workers.join_next().await {
            handle!(result, JoinNextFailed);
        }
        handle!(result, ServeFailed, address);
        Ok(ExitCode::SUCCESS)
    }
}

#[derive(Error, Debug)]
pub enum CommandRunError {
    #[error("failed to load CRS-Web configuration")]
    TryFromFailed { source: TryFromOptionPathBufForConfigError },
    #[error("failed to open CRS-Web server")]
    OpenServerFailed { source: ServerOpenError },
    #[error("failed to listen at '{address}'")]
    BindFailed { source: io::Error, address: SocketAddr },
    #[error("Codex synchronization task failed")]
    JoinNextFailed { source: JoinError },
    #[error("failed to serve CRS-Web at '{address}'")]
    ServeFailed { source: io::Error, address: SocketAddr },
}
