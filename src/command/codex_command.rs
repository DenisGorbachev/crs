use crate::{ThreadCodexCommand, ThreadCodexCommandRunError};
use CodexSubcommand::*;
use clap::{Parser, Subcommand};
use errgonomic::map_err;
use std::process::ExitCode;
use thiserror::Error;

#[derive(Parser, Clone, Debug)]
#[command(flatten_help = true)]
pub struct CodexCommand {
    #[command(subcommand)]
    subcommand: CodexSubcommand,
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
        match subcommand {
            Thread(command) => map_err!(command.run().await, ThreadCodexCommandRunFailed),
        }
    }
}

#[derive(Error, Debug)]
pub enum CodexCommandRunError {
    #[error("failed to run Codex thread command")]
    ThreadCodexCommandRunFailed { source: ThreadCodexCommandRunError },
}
