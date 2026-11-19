use crate::SharedServer;
use axum::{
    Extension,
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use dioxus::fullstack::ServerEvents;
use errgonomic::ErrorDisplayer;
use futures::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use tokio::{sync::watch, task::spawn_blocking};
use tracing::error;

#[derive(Deserialize, Serialize)]
pub struct ThreadEvents {
    pub source: String,
    pub id: String,
}

pub async fn thread_events(Extension(server): Extension<SharedServer>, Extension(mut shutdown): Extension<watch::Receiver<()>>, Query(query): Query<ThreadEvents>) -> Response {
    let (source, form_key) = {
        let server = server.lock().await;
        let Some(source) = server.sources.get(&query.source) else {
            return StatusCode::NOT_FOUND.into_response();
        };
        (source.clone(), server.form_key.to_string())
    };
    let store = source.store.clone();
    let config = source.config.clone();
    let id = query.id.clone();
    match spawn_blocking(move || store.record(&store.snapshot(), &config, &query.source, &query.id)).await {
        Ok(Ok(Some(_))) => (),
        Ok(Ok(None)) => return StatusCode::NOT_FOUND.into_response(),
        Ok(Err(error)) => {
            error!(%id, error = %ErrorDisplayer(&error), "failed to subscribe to thread");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        Err(error) => {
            error!(%id, %error, "thread subscription task failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
    let receiver = {
        let mut viewed = source.viewed.lock().await;
        viewed.retain(|_, sender| sender.receiver_count() != 0);
        viewed
            .entry(id)
            .or_insert_with(|| watch::channel(()).0)
            .subscribe()
    };
    let changes = stream::unfold(receiver, |mut receiver| async move { receiver.changed().await.ok().map(|()| (Ok(None), receiver)) });
    let events = stream::once(async move { Ok(Some(form_key)) })
        .chain(changes)
        .take_until(async move {
            let _ = shutdown.changed().await;
        });
    ServerEvents::from_stream(events).into_response()
}
