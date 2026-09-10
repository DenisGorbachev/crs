use crate::IntoItemsAndCursor;
pub use codex_thread_store::ItemPage;
use codex_thread_store::StoredThreadItem;

impl_into_items_and_cursor!(ItemPage, Vec<StoredThreadItem>, String);
