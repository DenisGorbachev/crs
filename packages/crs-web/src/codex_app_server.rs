use CodexAppServer::*;
use codex_app_server_client::{AppServerClient, DEFAULT_IN_PROCESS_CHANNEL_CAPACITY, RemoteAppServerClient, RemoteAppServerConnectArgs, RemoteAppServerEndpoint};
use codex_utils_absolute_path::AbsolutePathBuf;
use errgonomic::{handle, handle_bool, handle_discard, handle_opt};
use serde::{Deserialize, Serialize};
use std::env::var_os;
use std::io;
use std::path::PathBuf;
use thiserror::Error;
use url::Url;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "transport", rename_all = "kebab-case", deny_unknown_fields)]
pub enum CodexAppServer {
    UnixSocket {
        socket_path: PathBuf,
    },
    Websocket {
        url: Url,
        #[serde(skip_serializing_if = "Option::is_none")]
        auth_token_env: Option<String>,
    },
}

impl CodexAppServer {
    pub async fn connect(&self) -> Result<AppServerClient, CodexAppServerConnectError> {
        use CodexAppServerConnectError::*;
        let endpoint = match self {
            UnixSocket {
                socket_path,
            } => RemoteAppServerEndpoint::UnixSocket {
                socket_path: handle!(AbsolutePathBuf::relative_to_current_dir(socket_path), RelativeToCurrentDirFailed, socket_path: socket_path.as_path()),
            },
            Websocket {
                url,
                auth_token_env,
            } => {
                handle_bool!(!matches!(url.scheme(), "ws" | "wss") || url.host().is_none() || !url.username().is_empty() || url.password().is_some() || url.fragment().is_some(), UrlInvalid);
                let auth_token = match auth_token_env {
                    Some(name) => {
                        let value = handle_opt!(var_os(name), TokenEnvNotFound, name: name.as_str());
                        Some(handle_discard!(value.into_string(), TokenEnvInvalid, name: name.as_str()))
                    }
                    None => None,
                };
                RemoteAppServerEndpoint::WebSocket {
                    websocket_url: url.to_string(),
                    auth_token,
                }
            }
        };
        let client = handle!(
            RemoteAppServerClient::connect(RemoteAppServerConnectArgs {
                endpoint,
                client_name: env!("CARGO_PKG_NAME").into(),
                client_version: env!("CARGO_PKG_VERSION").into(),
                experimental_api: true,
                required_human_approval: false,
                mcp_server_openai_form_elicitation: false,
                opt_out_notification_methods: Vec::new(),
                channel_capacity: DEFAULT_IN_PROCESS_CHANNEL_CAPACITY,
            })
            .await,
            ConnectFailed
        );
        Ok(AppServerClient::Remote(client))
    }
}

#[derive(Error, Debug)]
pub enum CodexAppServerConnectError {
    #[error("failed to resolve Codex socket path '{}'", socket_path.display())]
    RelativeToCurrentDirFailed { source: io::Error, socket_path: PathBuf },
    #[error("invalid Codex WebSocket URL")]
    UrlInvalid {},
    #[error("Codex bearer token environment variable '{name}' is missing")]
    TokenEnvNotFound { name: String },
    #[error("Codex bearer token environment variable '{name}' is not valid UTF-8")]
    TokenEnvInvalid { name: String },
    #[error("failed to connect to the Codex app-server")]
    ConnectFailed { source: io::Error },
}
