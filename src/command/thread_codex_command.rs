use crate::{GetThreadCodexCommand, GetThreadCodexCommandRunError};
use ThreadCodexSubcommand::*;
use clap::{Parser, Subcommand};
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
    Get(GetThreadCodexCommand),
}

impl ThreadCodexCommand {
    pub async fn run(self) -> Result<ExitCode, ThreadCodexCommandRunError> {
        use ThreadCodexCommandRunError::*;
        let Self {
            subcommand,
        } = self;
        match subcommand {
            Get(command) => Ok(handle!(command.run().await, GetThreadCodexCommandRunFailed)),
        }
    }
}

#[derive(Error, Debug)]
pub enum ThreadCodexCommandRunError {
    #[error("failed to run a command on a Codex thread")]
    GetThreadCodexCommandRunFailed { source: GetThreadCodexCommandRunError },
}
