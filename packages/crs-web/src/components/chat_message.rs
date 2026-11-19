use crate::{Item, ItemContent};
use dioxus::prelude::*;
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

#[component]
pub fn ChatMessage(speaker: String, #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(article {
        class: "chat-message my-5",
        "data-response": speaker.clone(),
        aria_label: speaker,
    });
    rsx! {
        Item {
            attributes: merge_attributes(vec![base, attributes]),
            r#as: move |attrs: Vec<Attribute>| rsx! { article { ..attrs, ItemContent { {children.clone()} } } },
        }
    }
}
