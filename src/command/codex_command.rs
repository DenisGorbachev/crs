use crate::{ThreadCodexCommand, ThreadCodexCommandRunError};
use CodexSubcommand::*;
use clap::{Parser, Subcommand};
use codex_app_server_client::{DEFAULT_IN_PROCESS_CHANNEL_CAPACITY, RemoteAppServerClient, RemoteAppServerConnectArgs, RemoteAppServerEndpoint};
use codex_tui::resolve_remote_addr;
use errgonomic::handle;
use std::io;
use std::process::ExitCode;
use thiserror::Error;

#[derive(Parser, Clone, Debug)]
#[command(flatten_help = true)]
pub struct CodexCommand {
    #[arg(long, env = "CRS_CODEX_APP_SERVER", default_value = "unix://", value_parser = resolve_remote_addr, value_name = "URL")]
    pub app_server: RemoteAppServerEndpoint,
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
            app_server,
            subcommand,
        } = self;
        let args = RemoteAppServerConnectArgs {
            endpoint: app_server,
            client_name: env!("CARGO_PKG_NAME").into(),
            client_version: env!("CARGO_PKG_VERSION").into(),
            experimental_api: true,
            mcp_server_openai_form_elicitation: false,
            opt_out_notification_methods: Vec::new(),
            channel_capacity: DEFAULT_IN_PROCESS_CHANNEL_CAPACITY,
        };
        let connect_result = RemoteAppServerClient::connect(args.clone()).await;
        let mut client = handle!(connect_result, ConnectFailed, args);
        let result = match subcommand {
            Thread(command) => command.run(&mut client).await,
        };
        let shutdown_result = client.shutdown().await;
        let exit_code = handle!(result, ThreadCodexCommandRunFailed, shutdown_result);
        handle!(shutdown_result, ShutdownFailed, args);
        Ok(exit_code)
    }
}

#[derive(Error, Debug)]
pub enum CodexCommandRunError {
    #[error("failed to connect to the Codex app-server")]
    ConnectFailed { source: io::Error, args: Box<RemoteAppServerConnectArgs> },
    #[error("failed to run Codex thread command; connection shutdown: {shutdown_result:?}")]
    ThreadCodexCommandRunFailed { source: ThreadCodexCommandRunError, shutdown_result: io::Result<()> },
    #[error("failed to shut down the Codex app-server connection")]
    ShutdownFailed { source: io::Error, args: Box<RemoteAppServerConnectArgs> },
}
