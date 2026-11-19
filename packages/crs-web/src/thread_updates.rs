use crate::{Config, RenderTurnsError, SharedServer, ThreadHeader, ThreadStore, ThreadStorePageError, ThreadStoreRecordError, render_turns};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use dioxus::ssr::render_element;
use errgonomic::{ErrorDisplayer, handle};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::task::spawn_blocking;
use tracing::error;

#[derive(Deserialize, Serialize)]
pub struct ThreadUpdates {
    pub source: String,
    pub id: String,
    pub versions: Vec<String>,
    pub header: String,
}

impl ThreadUpdates {
    fn read(&self, store: &ThreadStore, config: &Config) -> Result<Response, ThreadUpdatesReadError> {
        use ThreadUpdatesReadError::*;
        let snapshot = store.snapshot();
        let Some(mut record) = handle!(store.record(&snapshot, config, &self.source, &self.id), RecordFailed) else {
            return Ok(StatusCode::NOT_FOUND.into_response());
        };
        record.thread.turns.clear();
        let header = handle!(ThreadHeader::try_from(&record), HeaderFailed);
        let header = (header.version != self.header).then_some(header);
        if record.turn_versions.as_ref() == Some(&self.versions) {
            return Ok(if header.is_some() {
                Json((self.versions.len(), Vec::<(usize, String)>::new(), header)).into_response()
            } else {
                StatusCode::NO_CONTENT.into_response()
            });
        }
        let page = handle!(store.page(&snapshot, &self.source, record), PageFailed);
        let versions = &page.versions;
        let turns = handle!(render_turns(page.turns, versions, &self.versions), RenderTurnsFailed);
        let html: Vec<_> = turns
            .into_iter()
            .map(|(index, turn)| (index, render_element(turn)))
            .collect();
        Ok(Json((versions.len(), html, header)).into_response())
    }
}

pub async fn thread_updates(Extension(server): Extension<SharedServer>, Json(request): Json<ThreadUpdates>) -> Response {
    let Some(source) = server.lock().await.sources.get(&request.source).cloned() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let id = request.id.clone();
    let store = source.store.clone();
    let config = source.config.clone();
    match spawn_blocking(move || request.read(&store, &config)).await {
        Ok(Ok(response)) => response,
        Ok(Err(error)) => {
            error!(%id, error = %ErrorDisplayer(&error), "failed to read thread updates");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Err(error) => {
            error!(%id, %error, "thread update task failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Error, Debug)]
pub enum ThreadUpdatesReadError {
    #[error("failed to encode thread header")]
    HeaderFailed { source: serde_json::Error },
    #[error("failed to read thread record")]
    RecordFailed { source: ThreadStoreRecordError },
    #[error("failed to read thread history")]
    PageFailed { source: ThreadStorePageError },
    #[error("failed to render thread updates")]
    RenderTurnsFailed { source: RenderTurnsError },
}
