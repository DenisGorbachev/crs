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

macro_rules! app_server_pages {
    ($client:expr, $params:expr, $request:ident, $response:ty) => {{
        use codex_app_server_protocol::{ClientRequest, RequestId};
        use uuid::Uuid;
        $crate::page_stream($client, $params, |client, params| {
            client.request_typed::<$response>(ClientRequest::$request {
                request_id: RequestId::String(Uuid::new_v4().to_string()),
                params,
            })
        })
    }};
}

mod print_command;

pub use print_command::*;
mod codex_command;
pub use codex_command::*;
mod thread_codex_command;
pub use thread_codex_command::*;
mod get_thread_codex_command;
pub use get_thread_codex_command::*;
mod render_agent_message_get_thread_codex_command;
pub use render_agent_message_get_thread_codex_command::*;
mod filter_thread_codex_command;
pub use filter_thread_codex_command::*;
