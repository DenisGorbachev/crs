use crate::icon;
use dioxus::prelude::*;
use dioxus_primitives::toggle::Toggle;
use dioxus_primitives::toolbar::{Toolbar, ToolbarButton};

#[component]
pub fn ThreadControls() -> Element {
    rsx! { Toolbar { class: "controls", aria_label: "Thread controls", hidden: true,
        for (index, (action, label, path)) in [
            ("first", "Beginning", "M5 4h14M6 14l6-6 6 6M12 8v12"),
            ("previous", "Previous message", "m6 15 6-6 6 6"),
            ("next", "Next message", "m6 9 6 6 6-6"),
            ("last", "End", "M5 20h14M6 10l6 6 6-6M12 16V4"),
        ].into_iter().enumerate() {
            ToolbarButton { index, class: "icon-button", "data-go": action, aria_label: label, title: label, {icon(path)} }
        }
        Toggle { class: "icon-button", id: "quote-mode", aria_label: "Quote", default_pressed: true,
            {icon("M10 6H4v6h5c0 3-2 5-5 6m16-12h-6v6h5c0 3-2 5-5 6")}
        }
    } }
}
