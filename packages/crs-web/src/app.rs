use crate::{Config, Route, SharedServer, ThreadSummary, UrlSegment};
use axum::Extension;
use dioxus::fullstack::FullstackContext;
use dioxus::prelude::*;
use dioxus::router::ParseRouteError;
use errgonomic::ErrorDisplayer;
use std::sync::Arc;
use tokio::task::spawn_blocking;
use tracing::error;

#[component]
pub fn App() -> Element {
    rsx! {
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1" }
        document::Style { {include_str!("thread_page.css")} }
        ErrorBoundary {
            handle_error: |context: ErrorContext| {
                let missing = context.error().is_some_and(|e| e.downcast_ref::<ParseRouteError>().is_some());
                let (status, title) = if missing { (StatusCode::NOT_FOUND, "Page not found") } else { (StatusCode::INTERNAL_SERVER_ERROR, "Could not display page") };
                page_error(status);
                rsx! { main { a { href: Route::Home {}.to_string(), "Servers" } h1 { "{title}" } } }
            },
            Router::<Route> {}
        }
    }
}

#[get("/api/threads", Extension(server): Extension<SharedServer>)]
pub async fn list_threads() -> Result<Vec<ThreadSummary>, ServerFnError> {
    let (store, config) = {
        let server = server.lock().await;
        (server.threads.clone(), server.config.clone())
    };
    match spawn_blocking(move || store.list(&config, config.codex_app_servers.keys().map(String::as_str))).await {
        Ok(Ok(threads)) => return Ok(threads),
        Ok(Err(error)) => error!(error = %ErrorDisplayer(&error), "failed to list saved threads"),
        Err(error) => error!(%error, "thread storage task failed"),
    }
    Err(ServerFnError::new("Could not read synchronized threads"))
}

#[component]
pub fn Home() -> Element {
    let config = use_context::<Arc<Config>>();
    rsx! { main {
        document::Title { "Codex servers · CRS-Web" }
        h1 { "Codex servers" }
        if config.codex_app_servers.is_empty() { p { "No Codex servers configured." } }
        ul {
            for name in config.codex_app_servers.keys() {
                li { Link { to: Route::Server { source: UrlSegment { value: name.clone() } }, "{name}" } }
            }
        }
    } }
}

pub fn page_error(status: StatusCode) {
    FullstackContext::commit_http_status(status, None);
}
