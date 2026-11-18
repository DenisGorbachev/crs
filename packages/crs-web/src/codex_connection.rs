use codex_app_server_client::{AppServerClient, AppServerRequestHandle, TypedRequestError};
use codex_app_server_protocol::ClientRequest;
use errgonomic::handle;
use serde::de::DeserializeOwned;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Notify;
use tokio::time::error::Elapsed;
use tokio::time::timeout;

pub struct CodexConnection {
    pub requests: AppServerRequestHandle,
    pub reconnect: Notify,
}

impl From<&AppServerClient> for CodexConnection {
    fn from(client: &AppServerClient) -> Self {
        Self {
            requests: client.request_handle(),
            reconnect: Notify::new(),
        }
    }
}

impl CodexConnection {
    pub async fn request<T: DeserializeOwned>(&self, request: ClientRequest) -> Result<T, CodexConnectionRequestError> {
        use CodexConnectionRequestError::*;
        use TypedRequestError::Transport;
        let method = request.method_name();
        let response = timeout(Duration::from_secs(30), self.requests.request_typed(request)).await;
        if matches!(&response, Err(_) | Ok(Err(Transport { .. }))) {
            self.reconnect.notify_one();
        }
        let result = handle!(response, TimeoutFailed, method);
        Ok(handle!(result, RequestTypedFailed, method))
    }
}

#[derive(Error, Debug)]
pub enum CodexConnectionRequestError {
    #[error("Codex request '{method}' timed out")]
    TimeoutFailed { source: Elapsed, method: &'static str },
    #[error("Codex request '{method}' failed")]
    RequestTypedFailed { source: Box<TypedRequestError>, method: &'static str },
}
