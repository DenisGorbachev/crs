use crate::{CodexThreadId, RenderFinalAnswerThreadCodexCommand, RenderFinalAnswerThreadCodexCommandRunError};
use ThreadCodexSubcommand::*;
use clap::{Parser, Subcommand};
use errgonomic::map_err;
use std::process::ExitCode;
use thiserror::Error;

#[derive(Parser, Clone, Debug)]
#[command(flatten_help = true)]
pub struct ThreadCodexCommand {
    #[arg(long, env = "CRS_CODEX_THREAD_ID", value_parser = CodexThreadId::from_string)]
    thread_id: CodexThreadId,
    #[command(subcommand)]
    subcommand: ThreadCodexSubcommand,
}

#[derive(Subcommand, Clone, Debug)]
pub enum ThreadCodexSubcommand {
    RenderFinalAnswer(RenderFinalAnswerThreadCodexCommand),
}

impl ThreadCodexCommand {
    pub async fn run(self) -> Result<ExitCode, ThreadCodexCommandRunError> {
        use ThreadCodexCommandRunError::*;
        let Self {
            thread_id,
            subcommand,
        } = self;
        match subcommand {
            RenderFinalAnswer(command) => map_err!(command.run(thread_id).await, RenderFinalAnswerThreadCodexCommandRunFailed),
        }
    }
}

#[derive(Error, Debug)]
pub enum ThreadCodexCommandRunError {
    #[error("failed to render the final answer from a Codex thread")]
    RenderFinalAnswerThreadCodexCommandRunFailed { source: RenderFinalAnswerThreadCodexCommandRunError },
}
