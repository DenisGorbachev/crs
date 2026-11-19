use dioxus::prelude::*;
use dioxus_primitives::collapsible::Collapsible;

#[component]
pub fn Disclosure(
    label: Element,
    #[props(default)] summary_class: String,
    #[props(extends = GlobalAttributes)]
    #[props(extends = details)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        Collapsible {
            attributes,
            r#as: move |mut attrs: Vec<Attribute>| {
                attrs.retain(|attr| attr.name != "data-open");
                rsx! { details { ..attrs,
                    summary { class: summary_class.clone(), {label.clone()} }
                    {children.clone()}
                } }
            },
        }
    }
}
