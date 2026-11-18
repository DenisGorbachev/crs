use crate::{Config, MessageForm, Route, ThreadHistory, ThreadRecord, ThreadStore, UrlSegment, page_error, thread_item_view, turn_anchor};
use codex_app_server_protocol::ThreadItem;
use dioxus::prelude::*;
use errgonomic::{ErrorDisplayer, handle};
use std::collections::HashMap;
use std::mem::take;
use std::sync::Arc;
use thiserror::Error;
use tokio::task::spawn_blocking;
use tracing::error;

pub struct ThreadPage {
    pub record: ThreadRecord,
    pub history: ThreadHistory,
}

impl ThreadPage {
    pub fn render(self, source: &str) -> Result<Element, ThreadPageRenderError> {
        use ThreadPageRenderError::*;
        let Self {
            record,
            history,
        } = self;
        let mut entries = HashMap::<_, Vec<_>>::new();
        for entry in history.items {
            let id = entry.turn_id.clone();
            entries
                .entry(id)
                .or_default()
                .push(handle!(ThreadItem::try_from(entry), DeferredOutputInvalid));
        }
        let mut turns = Vec::with_capacity(history.turns.len());
        for (number, mut turn) in (1usize..).zip(history.turns) {
            let items = take(&mut turn.items)
                .into_iter()
                .chain(entries.remove(&turn.id).into_iter().flatten());
            let mut transcript = Vec::new();
            for item in items {
                transcript.push(thread_item_view(handle!(serde_json::to_value(item), EncodeItemFailed)));
            }
            let mut metadata = handle!(serde_json::to_value(&turn), EncodeTurnFailed);
            if let Some(fields) = metadata.as_object_mut() {
                fields.remove("items");
            }
            turns.push(rsx! {
                section { id: turn_anchor(&turn.id),
                    h2 { "Turn {number}" }
                    if let Some(error) = &turn.error { p { role: "alert", "{error.message}" } }
                    details { summary { "Turn details · {turn.status:?}" } pre { "{metadata:#}" } }
                    if transcript.is_empty() { p { "No items in this turn." } }
                    {transcript.into_iter()}
                }
            });
        }
        let metadata = handle!(serde_json::to_string_pretty(&record), EncodeRecordFailed);
        let title = record.title();
        Ok(rsx! {
            document::Title { "{title} · CRS-Web" }
            h1 { "{title}" }
            p { code { "{record.thread.id}" } if record.archived { " · Archived" } }
            details { summary { "Thread details" } pre { "{metadata}" } }
            if turns.is_empty() { p { "No turns in this thread." } }
            {turns.into_iter()}
            if record.thread.can_accept_direct_input == Some(false) {
                p { "This thread does not accept direct messages." }
            } else {
                MessageForm { source: source.to_owned(), thread_id: record.thread.id.clone() }
            }
        })
    }
}

#[component]
pub fn ThreadView(source: ReadSignal<UrlSegment>, id: ReadSignal<UrlSegment>, anchor: String) -> Element {
    let store = use_context::<ThreadStore>();
    let config = use_context::<Arc<Config>>();
    let page = use_resource(move || {
        let store = store.clone();
        let config = config.clone();
        let name = source().value;
        let id = id().value;
        async move {
            let result = spawn_blocking({
                let name = name.clone();
                let id = id.clone();
                move || store.read(&config, &name, &id)
            })
            .await;
            match result {
                Ok(Ok(Some(page))) => match page.render(&name) {
                    Ok(content) => return (StatusCode::OK, content),
                    Err(error) => error!(%name, %id, error = %ErrorDisplayer(&error), "failed to render saved thread"),
                },
                Ok(Ok(None)) => return (StatusCode::NOT_FOUND, rsx! { h1 { "Thread not found" } }),
                Ok(Err(error)) => error!(%name, %id, error = %ErrorDisplayer(&error), "failed to read saved thread"),
                Err(error) => error!(%name, %id, %error, "thread storage task failed"),
            }
            (StatusCode::INTERNAL_SERVER_ERROR, rsx! { h1 { "Could not read thread" } })
        }
    });
    let ready = page.suspend()?;
    let data = ready.read();
    let (status, content) = &*data;
    if *status != StatusCode::OK {
        page_error(*status);
    }
    rsx! { main {
        nav {
            Link { to: Route::Home {}, "Servers" }
            " / "
            Link { to: Route::Server { source: source() }, "{source().value}" }
        }
        {content.clone()}
    } }
}

#[derive(Error, Debug)]
pub enum ThreadPageRenderError {
    #[error("saved thread contains deferred item output")]
    DeferredOutputInvalid { source: codex_app_server_protocol::DeferredItemOutput },
    #[error("failed to encode thread item")]
    EncodeItemFailed { source: serde_json::Error },
    #[error("failed to encode turn metadata")]
    EncodeTurnFailed { source: serde_json::Error },
    #[error("failed to encode thread metadata")]
    EncodeRecordFailed { source: serde_json::Error },
}
