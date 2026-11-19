use crate::{ChatMessage, Config, Disclosure, MessageForm, PageNav, ThreadHeader, ThreadRecord, ThreadStore, UrlSegment, page_error, thread_item_view, turn_anchor};
use codex_app_server_protocol::Turn;
use dioxus::prelude::*;
use errgonomic::{ErrorDisplayer, handle};
use std::mem::take;
use std::sync::Arc;
use thiserror::Error;
use tokio::task::spawn_blocking;
use tracing::error;

pub struct ThreadPage {
    pub versions: Vec<String>,
    pub record: ThreadRecord,
    pub turns: Vec<Turn>,
}

impl ThreadPage {
    pub fn render(self, source: &str) -> Result<Element, ThreadPageRenderError> {
        use ThreadPageRenderError::*;
        let Self {
            versions,
            mut record,
            turns,
        } = self;
        let turns = handle!(render_turns(turns, &versions, &[]), RenderTurnsFailed);
        let has_turns = !turns.is_empty();
        record.thread.turns.clear();
        let header = handle!(ThreadHeader::try_from(&record), EncodeRecordFailed);
        let title = &header.title;
        Ok(rsx! {
            document::Title { "{title} · CRS-Web" }
            h1 { id: "thread-title", "{title}" }
            p { code { "{record.thread.id}" } span { id: "archive-state", hidden: !header.archived, " · Archived" } }
            Disclosure { label: rsx! { "Thread details" }, pre { id: "thread-metadata", "data-version": header.version, "{header.metadata}" } }
            p { id: "update-status", role: "status", hidden: true, "Live updates paused. Retrying…" }
            p { id: "empty-transcript", hidden: has_turns, "No turns in this thread." }
            div { id: "turns", "data-source": source, "data-thread": record.thread.id.as_str(),
                {turns.into_iter().map(|(_, turn)| turn)}
            }
            div { id: "pending-messages", template {
                ChatMessage { speaker: "User", div { class: "user-bubble ms-auto box-border w-fit max-w-full whitespace-pre-wrap rounded-3xl bg-neutral-800 px-4 py-2.5 text-white scheme-dark" } }
            } }
            MessageForm { source: source.to_owned(), thread_id: record.thread.id.clone(), writable: header.writable }
        })
    }
}

pub fn render_turns(history: Vec<Turn>, versions: &[String], known: &[String]) -> Result<Vec<(usize, Element)>, RenderTurnsError> {
    use RenderTurnsError::*;
    let mut turns = Vec::new();
    for (number, ((index, mut turn), version)) in (1usize..).zip(history.into_iter().enumerate().zip(versions)) {
        if known.get(index) == Some(version) {
            continue;
        }

        let mut transcript = Vec::new();
        for item in take(&mut turn.items) {
            transcript.push(thread_item_view(handle!(serde_json::to_value(item), EncodeItemFailed)));
        }
        let mut metadata = handle!(serde_json::to_value(&turn), EncodeTurnFailed);
        if let Some(fields) = metadata.as_object_mut() {
            fields.remove("items");
        }
        turns.push((
            index,
            rsx! {
                section { id: turn_anchor(&turn.id), "data-version": version.as_str(),
                    h2 { "Turn {number}" }
                    if let Some(error) = &turn.error { p { role: "alert", "{error.message}" } }
                    Disclosure { label: rsx! { "Turn details · {turn.status:?}" }, pre { "{metadata:#}" } }
                    if transcript.is_empty() { p { "No visible items in this turn." } }
                    {transcript.into_iter()}
                }
            },
        ));
    }
    Ok(turns)
}

#[component]
pub fn ThreadView(source: ReadSignal<UrlSegment>, id: ReadSignal<UrlSegment>, anchor: String) -> Element {
    let store = use_context::<ThreadStore>();
    let config = use_context::<Arc<Config>>();
    let poll_ms = config.thread_refresh_interval().as_millis();
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
    rsx! { main { id: "thread-review", "data-poll-ms": "{poll_ms}",
        PageNav { source: source() }
        {content.clone()}
    } }
}

#[derive(Error, Debug)]
pub enum ThreadPageRenderError {
    #[error("failed to render thread turns")]
    RenderTurnsFailed { source: RenderTurnsError },
    #[error("failed to encode thread metadata")]
    EncodeRecordFailed { source: serde_json::Error },
}

#[derive(Error, Debug)]
pub enum RenderTurnsError {
    #[error("failed to encode thread item")]
    EncodeItemFailed { source: serde_json::Error },
    #[error("failed to encode turn metadata")]
    EncodeTurnFailed { source: serde_json::Error },
}
