use crate::{Home, ServerPage, ThreadView, UrlSegment};
use dioxus::prelude::*;

#[derive(Routable, Clone, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    Home {},
    #[route("/servers/:source", ServerPage)]
    Server { source: UrlSegment },
    #[route("/servers/:source/threads/:id", ThreadView)]
    Thread { source: UrlSegment, id: UrlSegment },
}
