use crate::IntoItemsAndCursor;
use codex_app_server_protocol::Thread;
pub use codex_app_server_protocol::ThreadListResponse;

impl_into_items_and_cursor!(ThreadListResponse, Vec<Thread>, String);
