use crate::{Route, UrlSegment};
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn MessageForm(source: String, thread_id: Option<String>) -> Element {
    let form_key = use_context::<Uuid>();
    let label = if thread_id.is_some() { "Message" } else { "First message" };
    let submit = if thread_id.is_some() { "Send message" } else { "Create thread" };
    rsx! { form { method: "post", action: "/messages",
        input { r#type: "hidden", name: "form_key", value: "{form_key}" }
        input { r#type: "hidden", name: "source", value: source }
        if let Some(id) = thread_id { input { r#type: "hidden", name: "thread_id", value: id } }
        label { r#for: "message", "{label}" }
        textarea { id: "message", name: "message", rows: "5", required: true }
        button { r#type: "submit", "{submit}" }
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
