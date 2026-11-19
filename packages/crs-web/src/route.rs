use crate::{Home, NewThreadPage, ServerPage, ThreadView, UrlSegment};
use dioxus::prelude::*;

#[derive(Routable, Clone, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    Home {},
    #[route("/servers/:source", ServerPage)]
    Server { source: UrlSegment },
    #[route("/servers/:source/threads/new", NewThreadPage)]
    NewThread { source: UrlSegment },
    #[route("/servers/:source/threads/:id#:anchor", ThreadView)]
    Thread { source: UrlSegment, id: UrlSegment, anchor: String },
}

impl Route {
    pub fn thread(source: UrlSegment, id: String, turn: Option<&str>) -> Self {
        Self::Thread {
            source,
            id: UrlSegment {
                value: id,
            },
            anchor: turn
                .map(|id| turn_anchor(id).replace('%', "%25"))
                .unwrap_or_default(),
        }
    }
}

pub fn turn_anchor(id: &str) -> String {
    format!("turn-{id}")
}
