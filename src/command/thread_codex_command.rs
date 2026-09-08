use crate::{CodexThreadId, RenderAgentMessageThreadCodexCommand, RenderAgentMessageThreadCodexCommandRunError};
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
    RenderAgentMessage(RenderAgentMessageThreadCodexCommand),
}

impl ThreadCodexCommand {
    pub async fn run(self) -> Result<ExitCode, ThreadCodexCommandRunError> {
        use ThreadCodexCommandRunError::*;
        let Self {
            thread_id,
            subcommand,
        } = self;
        match subcommand {
            RenderAgentMessage(command) => map_err!(command.run(thread_id).await, RenderAgentMessageThreadCodexCommandRunFailed),
        }
    }
}

#[derive(Error, Debug)]
pub enum ThreadCodexCommandRunError {
    #[error("failed to render an agent message from a Codex thread")]
    RenderAgentMessageThreadCodexCommandRunFailed { source: RenderAgentMessageThreadCodexCommandRunError },
}
