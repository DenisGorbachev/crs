use crate::{Config, Route, ThreadStore, UrlSegment, page_error};
use dioxus::prelude::*;
use errgonomic::ErrorDisplayer;
use std::iter::once;
use std::sync::Arc;
use tokio::task::spawn_blocking;
use tracing::error;

#[component]
pub fn ServerPage(source: ReadSignal<UrlSegment>) -> Element {
    let config = use_context::<Arc<Config>>();
    let store = use_context::<ThreadStore>();
    let threads = use_resource(move || {
        let config = config.clone();
        let store = store.clone();
        let name = source().value;
        async move {
            if !config.codex_app_servers.contains_key(&name) {
                return Err(StatusCode::NOT_FOUND);
            }
            let result = spawn_blocking({
                let name = name.clone();
                move || store.list(&config, once(name.as_str()))
            })
            .await;
            match result {
                Ok(Ok(threads)) => return Ok(threads),
                Ok(Err(error)) => error!(%name, error = %ErrorDisplayer(&error), "failed to list saved threads"),
                Err(error) => error!(%name, %error, "thread storage task failed"),
            }
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    });
    let ready = threads.suspend()?;
    let data = ready.read();
    rsx! { main {
        document::Title { "{source().value} · CRS-Web" }
        nav { Link { to: Route::Home {}, "Servers" } }
        match &*data {
            Ok(threads) => rsx! {
                h1 { "{source().value}" }
                if threads.is_empty() { p { "No synchronized threads yet." } }
                ul {
                    for thread in threads {
                        li { key: "{thread.id}",
                            Link { to: Route::Thread { source: source(), id: UrlSegment { value: thread.id.clone() } }, "{thread.title}" }
                            if thread.archived { " · Archived" }
                        }
                    }
                }
            },
            Err(status) => {
                page_error(*status);
                rsx! { h1 { if *status == StatusCode::NOT_FOUND { "Server not found" } else { "Could not read threads" } } }
            }
        }
    } }
}
