use crate::{SharedServer, ThreadStore, ThreadSummary};
use axum::Extension;
use dioxus::prelude::*;
use errgonomic::ErrorDisplayer;
use tokio::task::spawn_blocking;
use tracing::error;

#[component]
pub fn App() -> Element {
    let threads = use_server_future(list_threads)?;
    let threads = threads.read();
    rsx! {
        main {
            h1 { "CRS-Web" }
            match threads.as_ref() {
                Some(Ok(threads)) if threads.is_empty() => rsx! { p { "No synchronized threads yet." } },
                Some(Ok(threads)) => rsx! {
                    ul {
                        for thread in threads {
                            li { key: "{thread.server_id}:{thread.id}",
                                strong { "{thread.title}" }
                                " · {thread.server_id}"
                                if thread.archived {
                                    " · Archived"
                                }
                            }
                        }
                    }
                },
                Some(Err(_)) => rsx! { p { role: "alert", "Could not read synchronized threads." } },
                None => rsx! { p { "Loading threads…" } },
            }
        }
    }
}

#[get("/api/threads", Extension(server): Extension<SharedServer>)]
pub async fn list_threads() -> Result<Vec<ThreadSummary>, ServerFnError> {
    let (db, config) = {
        let server = server.lock().await;
        (server.db.clone(), server.config.clone())
    };
    match spawn_blocking(move || ThreadStore::list(&db, &config)).await {
        Ok(Ok(threads)) => return Ok(threads),
        Ok(Err(error)) => error!(error = %ErrorDisplayer(&error), "failed to list saved threads"),
        Err(error) => error!(%error, "thread storage task failed"),
    }
    Err(ServerFnError::new("Could not read synchronized threads"))
}
