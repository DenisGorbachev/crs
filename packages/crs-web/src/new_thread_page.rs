use crate::{Config, MessageForm, PageNav, UrlSegment, page_error};
use dioxus::prelude::*;
use std::sync::Arc;

#[component]
pub fn NewThreadPage(source: UrlSegment) -> Element {
    let config = use_context::<Arc<Config>>();
    if !config.codex_app_servers.contains_key(&source.value) {
        page_error(StatusCode::NOT_FOUND);
        return rsx! { main { PageNav {} h1 { "Server not found" } } };
    }
    rsx! { main {
        document::Title { "New thread · {source.value} · CRS-Web" }
        PageNav { source: source.clone() }
        h1 { "New thread" }
        MessageForm { source: source.value }
    } }
}
