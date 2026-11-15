use crate::SetNextCursor;
pub use codex_app_server_protocol::ThreadListParams;

impl SetNextCursor for ThreadListParams {
    type Cursor = String;

    fn set_next_cursor(&mut self, cursor: Self::Cursor) {
        self.cursor = Some(cursor);
    }
}
