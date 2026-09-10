use crate::{FilterThreadCodexCommand, FilterThreadCodexCommandRunError, GetThreadCodexCommand, GetThreadCodexCommandRunError};
use ThreadCodexSubcommand::*;
use clap::{Parser, Subcommand};
use codex_thread_store::ThreadStore;
use errgonomic::handle;
use std::process::ExitCode;
use thiserror::Error;

#[derive(Parser, Clone, Debug)]
#[command(flatten_help = true)]
pub struct ThreadCodexCommand {
    #[command(subcommand)]
    pub subcommand: ThreadCodexSubcommand,
}

#[derive(Subcommand, Clone, Debug)]
pub enum ThreadCodexSubcommand {
    Filter(FilterThreadCodexCommand),
    Get(GetThreadCodexCommand),
}

impl ThreadCodexCommand {
    pub async fn run(self, store: &(impl ThreadStore + ?Sized)) -> Result<ExitCode, ThreadCodexCommandRunError> {
        use ThreadCodexCommandRunError::*;
        let Self {
            subcommand,
        } = self;
        match subcommand {
            Filter(command) => Ok(handle!(command.run(store).await, FilterThreadCodexCommandRunFailed)),
            Get(command) => Ok(handle!(command.run(store).await, GetThreadCodexCommandRunFailed)),
        }
    }
}

#[derive(Error, Debug)]
pub enum ThreadCodexCommandRunError {
    #[error("failed to filter Codex threads")]
    FilterThreadCodexCommandRunFailed { source: FilterThreadCodexCommandRunError },
    #[error("failed to run a command on a Codex thread")]
    GetThreadCodexCommandRunFailed { source: GetThreadCodexCommandRunError },
}
