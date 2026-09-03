use Subcommand::*;
use errgonomic::map_err;
use std::process::ExitCode;
use thiserror::Error;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, propagate_version = true, flatten_help = true, disable_help_subcommand = true)]
pub struct Command {
    #[command(subcommand)]
    subcommand: Subcommand,
}

#[derive(clap::Subcommand, Clone, Debug)]
pub enum Subcommand {
    Codex(CodexCommand),
    Print(PrintCommand),
}

impl Command {
    pub async fn run(self) -> Result<ExitCode, CommandRunError> {
        use CommandRunError::*;
        let Self {
            subcommand,
        } = self;
        match subcommand {
            Codex(command) => map_err!(command.run().await, CodexCommandRunFailed),
            Print(command) => map_err!(command.run().await, PrintCommandRunFailed),
        }
    }
}

#[derive(Error, Debug)]
pub enum CommandRunError {
    #[error("failed to run Codex command")]
    CodexCommandRunFailed { source: CodexCommandRunError },
    #[error("failed to run print command")]
    PrintCommandRunFailed { source: PrintCommandRunError },
}

mod print_command;

pub use print_command::*;
mod codex_command;
pub use codex_command::*;
mod thread_codex_command;
pub use thread_codex_command::*;
mod render_final_answer_thread_codex_command;
pub use render_final_answer_thread_codex_command::*;
