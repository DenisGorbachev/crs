use crate::icon;
use dioxus::prelude::*;

#[component]
pub fn ThreadControls() -> Element {
    rsx! { div { class: "controls", hidden: true,
        nav { class: "flex gap-1", aria_label: "Message navigation",
            for (action, label, path) in [
                ("first", "Beginning", "M5 4h14M6 14l6-6 6 6M12 8v12"),
                ("previous", "Previous message", "m6 15 6-6 6 6"),
                ("next", "Next message", "m6 9 6 6 6-6"),
                ("last", "End", "M5 20h14M6 10l6 6 6-6M12 16V4"),
            ] {
                button { class: "icon-button", r#type: "button", "data-go": action, aria_label: label, title: label, {icon(path)} }
            }
        }
        button { class: "icon-button", r#type: "button", id: "quote-mode", aria_label: "Quote", aria_pressed: "true",
            {icon("M10 6H4v6h5c0 3-2 5-5 6m16-12h-6v6h5c0 3-2 5-5 6")}
        }
    } }
}
