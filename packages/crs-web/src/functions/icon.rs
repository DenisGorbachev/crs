use dioxus::prelude::*;

pub fn icon(path: &str) -> Element {
    rsx! { svg {
        class: "size-4", view_box: "0 0 24 24", fill: "none", stroke: "currentColor",
        stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true",
        path { d: path }
    } }
}
