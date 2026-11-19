use crate::{Button, ButtonSize, ButtonVariant, ChatMessage, Textarea, icon};
use dioxus::prelude::*;

#[component]
pub fn QueuePanel() -> Element {
    rsx! { div { id: "pending-messages",
        template {
            ChatMessage { speaker: "User",
                div { class: "user-bubble whitespace-pre-wrap rounded-3xl bg-neutral-800 px-4 py-2.5 text-white scheme-dark" }
                div { class: "flex items-center justify-end gap-1",
                    small { "data-queue-state": "" }
                    a { hidden: true, download: "codex-queued-input.json", "Download original input" }
                    span { "data-queue-actions": "",
                        for (action, label, path) in [
                            ("steer", "Steer", "M4 19v-5a5 5 0 0 1 5-5h11m-5-5 5 5-5 5"),
                            ("edit", "Edit", "m4 16-1 5 5-1L20 8l-4-4ZM14 6l4 4"),
                            ("cancel", "Cancel", "m6 6 12 12M18 6 6 18"),
                        ] {
                            Button { r#type: "button", variant: ButtonVariant::Ghost, size: ButtonSize::IconXs, "data-queue-action": action, aria_label: label, title: label, {icon(path)} }
                        }
                    }
                }
                form { hidden: true, "data-queue-editor": "",
                    Textarea { aria_label: "Edit queued message", rows: "3", required: true }
                    div { class: "flex justify-end gap-1",
                        Button { r#type: "button", variant: ButtonVariant::Ghost, size: ButtonSize::Sm, "data-queue-action": "discard", "Discard edit" }
                        Button { r#type: "submit", size: ButtonSize::Sm, "Save edit" }
                    }
                }
            }
        }
    } }
}
