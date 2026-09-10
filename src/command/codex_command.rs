use crate::{ThreadCodexCommand, ThreadCodexCommandRunError};
use CodexSubcommand::*;
use clap::{Parser, Subcommand};
use codex_core::config::Config;
use codex_rollout::state_db::get_state_db;
use codex_thread_store::{LocalThreadStore, LocalThreadStoreConfig};
use errgonomic::handle;
use std::io;
use std::process::ExitCode;
use thiserror::Error;

#[derive(Parser, Clone, Debug)]
#[command(flatten_help = true)]
pub struct CodexCommand {
    #[command(subcommand)]
    pub subcommand: CodexSubcommand,
}

#[derive(Subcommand, Clone, Debug)]
pub enum CodexSubcommand {
    Thread(ThreadCodexCommand),
}

impl CodexCommand {
    pub async fn run(self) -> Result<ExitCode, CodexCommandRunError> {
        use CodexCommandRunError::*;
        let Self {
            subcommand,
        } = self;
        let config = handle!(Config::load_with_cli_overrides(Vec::new()).await, LoadWithCliOverridesFailed);
        let state_db = get_state_db(&config).await;
        let store = LocalThreadStore::new(LocalThreadStoreConfig::from_config(&config), state_db);
        match subcommand {
            Thread(command) => Ok(handle!(command.run(&store).await, ThreadCodexCommandRunFailed)),
        }
    }
}

#[derive(Error, Debug)]
pub enum CodexCommandRunError {
    #[error("failed to load the Codex configuration")]
    LoadWithCliOverridesFailed { source: io::Error },
    #[error("failed to run Codex thread command")]
    ThreadCodexCommandRunFailed { source: ThreadCodexCommandRunError },
}
