use crate::thread_store_max_page_size;
use codex_protocol::ThreadId;
use codex_thread_store::ItemSortKey::*;
use codex_thread_store::ListItemsParams;
use codex_thread_store::SortDirection::*;

pub fn list_items_params_all_reverse(thread_id: ThreadId) -> ListItemsParams {
    ListItemsParams {
        thread_id,
        turn_id: None,
        include_archived: true,
        cursor: None,
        page_size: thread_store_max_page_size(),
        sort_direction: Desc,
        sort_key: CreatedAtOrdinal,
        after_updated_at_ordinal: None,
    }
}
