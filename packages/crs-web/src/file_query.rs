use crate::{FileLinks, SharedServer, UrlSegment};
use axum::Extension;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::http::header::{CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS};
use axum::response::{IntoResponse, Response};
use base64::{Engine, engine::general_purpose::STANDARD};
use codex_app_server_protocol::{ClientRequest, FsReadFileParams, FsReadFileResponse, RequestId};
use codex_utils_absolute_path::AbsolutePathBuf;
use errgonomic::ErrorDisplayer;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tokio::task::spawn_blocking;
use tracing::error;
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
pub struct FileQuery {
    pub thread: String,
    pub path: AbsolutePathBuf,
}

pub async fn open_file(Extension(server): Extension<SharedServer>, Path(source): Path<String>, Query(file): Query<FileQuery>) -> Response {
    let Ok(source) = UrlSegment::from_str(&source) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(sync) = server.lock().await.sources.get(&source.value).cloned() else {
        return (StatusCode::NOT_FOUND, "Server not found.").into_response();
    };
    let lookup = sync.clone();
    let path = file.path.clone();
    let linked = spawn_blocking(move || {
        lookup
            .store
            .read(&lookup.config, &lookup.server_id, &file.thread)
            .map(|page| page.is_some_and(|page| FileLinks::from((lookup.server_id.as_str(), &page.record.thread)).contains(&page.turns, &path)))
    })
    .await;
    match linked {
        Ok(Ok(true)) => (),
        Ok(Ok(false)) => return (StatusCode::NOT_FOUND, "File link not found in this thread.").into_response(),
        Ok(Err(error)) => {
            error!(error = %ErrorDisplayer(&error), "could not read file link from thread");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        Err(error) => {
            error!(%error, "file link lookup task failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
    let Some(client) = sync.connection.lock().await.clone() else {
        return (StatusCode::SERVICE_UNAVAILABLE, "Codex server is disconnected.").into_response();
    };
    let request = ClientRequest::FsReadFile {
        request_id: RequestId::String(Uuid::new_v4().to_string()),
        params: FsReadFileParams {
            path: file.path,
        },
    };
    let response = match client.request::<FsReadFileResponse>(request).await {
        Ok(response) => response,
        Err(error) => return (StatusCode::BAD_GATEWAY, ErrorDisplayer(&error).to_string()).into_response(),
    };
    match STANDARD.decode(response.data_base64) {
        Ok(bytes) => (
            [
                (CONTENT_TYPE, "text/plain; charset=utf-8"),
                (X_CONTENT_TYPE_OPTIONS, "nosniff"),
                (CONTENT_SECURITY_POLICY, "default-src 'none'; sandbox"),
                (CACHE_CONTROL, "no-store"),
            ],
            bytes,
        )
            .into_response(),
        Err(error) => {
            error!(%error, "invalid base64 in fs/readFile response");
            (StatusCode::BAD_GATEWAY, "Codex returned invalid file data.").into_response()
        }
    }
}
