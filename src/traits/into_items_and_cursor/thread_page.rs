use crate::IntoItemsAndCursor;
use codex_thread_store::StoredThread;
pub use codex_thread_store::ThreadPage;

impl_into_items_and_cursor!(ThreadPage, Vec<StoredThread>, String);
