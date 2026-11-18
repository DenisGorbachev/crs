use crate::{Route, UrlSegment};
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn MessageForm(source: String, thread_id: Option<String>) -> Element {
    let form_key = use_context::<Uuid>();
    let existing = thread_id.is_some();
    let label = if existing { "Message" } else { "First message" };
    let submit = if existing { "Send message" } else { "Create thread" };
    rsx! { form { method: "post", action: "/messages", class: existing.then_some("thread-draft"),
        input { r#type: "hidden", name: "form_key", value: "{form_key}" }
        input { r#type: "hidden", name: "source", value: source }
        if let Some(id) = thread_id {
            input { r#type: "hidden", name: "thread_id", value: id }
            div { class: "quote-controls", hidden: true,
                button { r#type: "button", id: "quote-mode", aria_pressed: "false", aria_describedby: "quote-hint", "Quote" }
                small { id: "quote-hint", "Tap once for one quote, twice for multiple." }
                label { input { r#type: "checkbox", id: "quote-default" } " Quote by default" }
            }
        }
        label { r#for: "message", "{label}" }
        textarea { id: "message", name: "message", rows: "5", required: true }
        div { class: "draft-actions",
            button { r#type: "submit", "{submit}" }
            span { role: "status", "aria-live": "polite" }
        }
    } }
}

#[component]
pub fn MessageSent(source: UrlSegment, id: UrlSegment, turn: UrlSegment) -> Element {
    rsx! { main {
        h1 { "Message sent" }
        p { "Codex accepted your message. The saved history is catching up." }
        Link { to: Route::thread(source, id.value, Some(&turn.value)), "Open thread" }
    } }
}
