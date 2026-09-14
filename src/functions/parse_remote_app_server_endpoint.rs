use codex_app_server_client::RemoteAppServerEndpoint;
use codex_app_server_client::RemoteAppServerEndpoint::*;
use codex_utils_absolute_path::AbsolutePathBuf;
use errgonomic::{handle, handle_bool, handle_opt};
use std::io;
use std::path::Path;
use thiserror::Error;
use url::{ParseError, Url};

pub fn parse_remote_app_server_endpoint(input: &str) -> Result<RemoteAppServerEndpoint, ParseRemoteAppServerEndpointError> {
    use ParseRemoteAppServerEndpointError::*;
    let url = handle!(Url::parse(input), ParseFailed, input);
    handle_bool!(!url.username().is_empty() || url.password().is_some(), CredentialsInvalid);
    handle_bool!(url.fragment().is_some(), FragmentInvalid, url);
    match url.scheme() {
        "ws" | "wss" => Ok(WebSocket {
            websocket_url: url.into(),
            auth_token: None,
        }),
        "unix" => {
            let path = handle_opt!(input.strip_prefix("unix://"), UnixLocatorInvalid, url);
            handle_bool!(url.has_host() || url.port().is_some() || url.query().is_some() || !Path::new(path).is_absolute() || path.contains('\0'), UnixLocatorInvalid, url);
            let socket_path = handle!(AbsolutePathBuf::from_absolute_path_checked(path), FromAbsolutePathCheckedFailed, input);
            Ok(UnixSocket {
                socket_path,
            })
        }
        _ => Err(SchemeInvalid {
            url,
        }),
    }
}

#[derive(Error, Debug)]
pub enum ParseRemoteAppServerEndpointError {
    #[error("invalid app-server URL '{input}'")]
    ParseFailed { source: ParseError, input: String },
    #[error("app-server URL credentials are not supported")]
    CredentialsInvalid {},
    #[error("app-server URL '{url}' must not contain a fragment")]
    FragmentInvalid { url: Url },
    #[error("Unix app-server locator '{url}' must have an absolute path and no authority or query")]
    UnixLocatorInvalid { url: Url },
    #[error("failed to construct an absolute socket path from '{input}'")]
    FromAbsolutePathCheckedFailed { source: io::Error, input: String },
    #[error("unsupported app-server URL '{url}'; expected ws://, wss://, or unix://")]
    SchemeInvalid { url: Url },
}
