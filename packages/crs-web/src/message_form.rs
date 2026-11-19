use crate::{Route, UrlSegment, icon};
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
                button { class: "icon-button", r#type: "button", id: "quote-mode", title: "Quote", aria_label: "Quote", aria_pressed: "false", aria_describedby: "quote-hint",
                    {icon("M10 6H4v6h5c0 3-2 5-5 6m16-12h-6v6h5c0 3-2 5-5 6")}
                }
                small { id: "quote-hint" }
                label { input { r#type: "checkbox", id: "quote-default" } " Quote by default" }
            }
        }
        label { r#for: "message", "{label}" }
        textarea { id: "message", name: "message", rows: "5", required: true }
        div { class: "draft-actions",
            button { class: "icon-button primary", r#type: "submit", title: submit, aria_label: submit, {icon("m6 12 6-6 6 6M12 6v13")} }
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
