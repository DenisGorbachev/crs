use crate::IntoItemsAndCursor;
use codex_app_server_protocol::ThreadItemEntry;
pub use codex_app_server_protocol::ThreadItemsListResponse;

impl_into_items_and_cursor!(ThreadItemsListResponse, Vec<ThreadItemEntry>, String);
