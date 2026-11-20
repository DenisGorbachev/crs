use crate::{Home, NewThreadPage, ServerPage, ThreadView, UrlSegment};
use dioxus::prelude::*;
use url::form_urlencoded::byte_serialize;

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
        Self::thread_at(source, id, turn.map(turn_anchor).as_deref())
    }

    pub fn thread_at(source: UrlSegment, id: String, anchor: Option<&str>) -> Self {
        Self::Thread {
            source,
            id: UrlSegment {
                value: id,
            },
            anchor: anchor
                .map(|value| value.replace('%', "%25"))
                .unwrap_or_default(),
        }
    }
}

pub fn turn_anchor(id: &str) -> String {
    format!("turn-{id}")
}

pub fn item_anchor(turn_id: &str, item_id: &str) -> String {
    let segment = |id: &str| byte_serialize(id.as_bytes()).collect::<String>();
    format!("item-{}/{}", segment(turn_id), segment(item_id))
}
