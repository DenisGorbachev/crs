use crate::{CodexThreadId, RenderAgentMessageGetThreadCodexCommand, RenderAgentMessageGetThreadCodexCommandRunError};
use GetThreadCodexSubcommand::*;
use clap::{Parser, Subcommand};
use errgonomic::handle;
use std::process::ExitCode;
use thiserror::Error;

#[derive(Parser, Clone, Debug)]
#[command(flatten_help = true)]
pub struct GetThreadCodexCommand {
    /// The Codex thread to operate on.
    #[arg(value_name = "THREAD-ID", env = "CRS_CODEX_THREAD_ID", value_parser = CodexThreadId::from_string)]
    pub thread_id: CodexThreadId,
    #[command(subcommand)]
    pub subcommand: GetThreadCodexSubcommand,
}

#[derive(Subcommand, Clone, Debug)]
pub enum GetThreadCodexSubcommand {
    RenderAgentMessage(RenderAgentMessageGetThreadCodexCommand),
}

impl GetThreadCodexCommand {
    pub async fn run(self) -> Result<ExitCode, GetThreadCodexCommandRunError> {
        use GetThreadCodexCommandRunError::*;
        let Self {
            thread_id,
            subcommand,
        } = self;
        match subcommand {
            RenderAgentMessage(command) => Ok(handle!(command.run(thread_id).await, RenderAgentMessageGetThreadCodexCommandRunFailed, thread_id)),
        }
    }
}

#[derive(Error, Debug)]
pub enum GetThreadCodexCommandRunError {
    #[error("failed to render an agent message from Codex thread '{thread_id}'")]
    RenderAgentMessageGetThreadCodexCommandRunFailed { source: RenderAgentMessageGetThreadCodexCommandRunError, thread_id: CodexThreadId },
}
