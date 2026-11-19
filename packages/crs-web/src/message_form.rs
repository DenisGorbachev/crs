use crate::{Button, ButtonSize, Textarea, ThreadControls, icon};
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn MessageForm(source: String, thread_id: Option<String>, #[props(default = true)] writable: bool) -> Element {
    let form_key = use_context::<Uuid>();
    let existing = thread_id.is_some();
    let label = if existing { "Message" } else { "First message" };
    let submit = if existing { "Send message" } else { "Create thread" };
    rsx! { form { method: "post", action: "/messages", class: existing.then_some("thread-draft"),
        input { r#type: "hidden", name: "form_key", value: "{form_key}" }
        input { r#type: "hidden", name: "source", value: source }
        if let Some(id) = thread_id {
            input { r#type: "hidden", name: "thread_id", value: id }
            ThreadControls {}
            p { id: "direct-input-notice", hidden: writable, "This thread does not accept direct messages." }
        }
        fieldset { disabled: !writable, hidden: !writable,
            Textarea { aria_label: label, id: "message", name: "message", rows: "5", required: true }
            div { class: "draft-actions",
                Button { size: ButtonSize::IconSm, class: "rounded-full", r#type: "submit", title: submit, aria_label: submit, {icon("m6 12 6-6 6 6M12 6v13")} }
                span { role: "status", "aria-live": "polite" }
            }
        }
    } }
}
