use crate::{App, CodexSync, Server, ServerOpenError, SharedServer, ThreadStore, ThreadStoreOpenError};
use axum::{Extension, Router, serve};
use clap::Parser;
use dioxus::cli_config::fullstack_address_or_localhost;
use dioxus::server::{DioxusRouterExt, ServeConfig};
use errgonomic::handle;
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
    #[arg(long, env = "CRS_WEB_CONFIG")]
    config: PathBuf,
}

impl Command {
    pub async fn run(self) -> Result<ExitCode, CommandRunError> {
        use CommandRunError::*;
        let server = handle!(Server::open(self.config), OpenServerFailed);
        let store = handle!(ThreadStore::open(&server), OpenThreadStoreFailed);
        let config = server.config.clone();
        let server: SharedServer = Arc::new(Mutex::new(server));
        let router = Router::new()
            .serve_api_application(ServeConfig::new(), App)
            .layer(Extension(server));
        let address = fullstack_address_or_localhost();
        let listener = handle!(TcpListener::bind(address).await, BindFailed, address);
        let (shutdown, receiver) = watch::channel(());
        let mut workers = JoinSet::new();
        config.codex_app_servers.iter().for_each(|(id, source)| {
            workers.spawn(
                CodexSync {
                    server_id: id.clone(),
                    source: source.clone(),
                    config: config.clone(),
                    store: store.clone(),
                }
                .run(receiver.clone()),
            );
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
    #[error("failed to open CRS-Web server")]
    OpenServerFailed { source: ServerOpenError },
    #[error("failed to open CRS-Web thread storage")]
    OpenThreadStoreFailed { source: ThreadStoreOpenError },
    #[error("failed to listen at '{address}'")]
    BindFailed { source: io::Error, address: SocketAddr },
    #[error("Codex synchronization task failed")]
    JoinNextFailed { source: JoinError },
    #[error("failed to serve CRS-Web at '{address}'")]
    ServeFailed { source: io::Error, address: SocketAddr },
}
