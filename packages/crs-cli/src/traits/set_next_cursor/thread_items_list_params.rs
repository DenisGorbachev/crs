use crate::SetNextCursor;
use codex_app_server_protocol::ThreadItemsListCursor::Opaque;
pub use codex_app_server_protocol::ThreadItemsListParams;

impl SetNextCursor for ThreadItemsListParams {
    type Cursor = String;

    fn set_next_cursor(&mut self, cursor: Self::Cursor) {
        self.cursor = Some(Opaque(cursor));
    }
}
