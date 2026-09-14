use crate::CodexThreadId;
use codex_app_server_protocol::SortDirection::*;
use codex_app_server_protocol::ThreadItemsListParams;

pub fn list_items_params_all_reverse(thread_id: CodexThreadId) -> ThreadItemsListParams {
    ThreadItemsListParams {
        limit: Some(u32::MAX),
        cursor: None,
        sort_direction: Some(Desc),
        thread_id: thread_id.to_string(),
        turn_id: None,
    }
}
