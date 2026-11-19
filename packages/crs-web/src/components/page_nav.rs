use crate::{Route, UrlSegment};
use dioxus::prelude::*;
use dioxus_primitives::navbar::Navbar;

#[component]
pub fn PageNav(source: Option<UrlSegment>) -> Element {
    rsx! {
        Navbar { aria_label: "Breadcrumb", class: "page-nav",
            Link { to: Route::Home {}, role: "menuitem", "Servers" }
            if let Some(source) = source {
                span { aria_hidden: "true", " / " }
                Link { to: Route::Server { source: source.clone() }, role: "menuitem", "{source.value}" }
            }
        }
    }
}
