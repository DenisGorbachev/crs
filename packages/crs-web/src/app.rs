use crate::{Config, Item, ItemContent, ItemGroup, ItemSize, ItemTitle, Route, UrlSegment};
use dioxus::fullstack::FullstackContext;
use dioxus::prelude::*;
use dioxus::router::ParseRouteError;
use std::sync::Arc;

#[component]
pub fn App() -> Element {
    rsx! {
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1, interactive-widget=resizes-content" }
        document::Style { {include_str!("../assets/tailwind.css")} }
        script { r#type: "module", src: "/thread-review.js" }
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

#[component]
pub fn Home() -> Element {
    let config = use_context::<Arc<Config>>();
    rsx! { main {
        document::Title { "Codex servers · CRS-Web" }
        h1 { "Codex servers" }
        if config.codex_app_servers.is_empty() { p { "No Codex servers configured." } }
        ItemGroup {
            for name in config.codex_app_servers.keys() {
                Item { key: "{name}", role: "listitem", size: ItemSize::Sm,
                    ItemContent { ItemTitle { Link { to: Route::Server { source: UrlSegment { value: name.clone() } }, "{name}" } } }
                }
            }
        }
    } }
}

pub fn page_error(status: StatusCode) {
    FullstackContext::commit_http_status(status, None);
}
