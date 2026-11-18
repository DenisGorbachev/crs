use CodexAppServer::*;
use codex_app_server_client::{AppServerClient, DEFAULT_IN_PROCESS_CHANNEL_CAPACITY, RemoteAppServerClient, RemoteAppServerConnectArgs, RemoteAppServerEndpoint, app_server_control_socket_path};
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_home_dir::find_codex_home;
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
        #[serde(skip_serializing_if = "Option::is_none")]
        socket_path: Option<PathBuf>,
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
                socket_path: match socket_path {
                    Some(path) => handle!(AbsolutePathBuf::relative_to_current_dir(path), RelativeToCurrentDirFailed, socket_path: path.as_path()),
                    None => {
                        let home = handle!(find_codex_home(), FindCodexHomeFailed);
                        handle!(app_server_control_socket_path(&home), AppServerControlSocketPathFailed, home: home.to_path_buf())
                    }
                },
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
        let args = RemoteAppServerConnectArgs {
            endpoint,
            client_name: env!("CARGO_PKG_NAME").into(),
            client_version: env!("CARGO_PKG_VERSION").into(),
            experimental_api: true,
            required_human_approval: false,
            mcp_server_openai_form_elicitation: false,
            opt_out_notification_methods: Vec::new(),
            channel_capacity: DEFAULT_IN_PROCESS_CHANNEL_CAPACITY,
        };
        #[cfg(windows)]
        if matches!(
            self,
            UnixSocket {
                socket_path: None
            }
        ) {
            return Ok(AppServerClient::Remote(handle!(RemoteAppServerClient::connect_local_daemon(args).await, ConnectFailed)));
        }
        let client = handle!(RemoteAppServerClient::connect(args).await, ConnectFailed);
        Ok(AppServerClient::Remote(client))
    }
}

#[derive(Error, Debug)]
pub enum CodexAppServerConnectError {
    #[error("failed to locate Codex home")]
    FindCodexHomeFailed { source: io::Error },
    #[error("failed to locate Codex control socket under '{}'", home.display())]
    AppServerControlSocketPathFailed { source: io::Error, home: PathBuf },
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
