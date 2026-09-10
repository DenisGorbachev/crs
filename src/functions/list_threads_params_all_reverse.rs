use crate::{ListThreadsParams, thread_store_max_page_size};
use codex_thread_store::SortDirection::*;
use codex_thread_store::ThreadSortKey::*;
use std::path::PathBuf;

pub fn list_threads_params_all_reverse(cwd_filters: Option<Vec<PathBuf>>, search_term: Option<String>) -> ListThreadsParams {
    ListThreadsParams {
        page_size: thread_store_max_page_size(),
        cursor: None,
        sort_key: CreatedAt,
        sort_direction: Desc,
        allowed_sources: vec![],
        model_providers: None,
        cwd_filters,
        section: None,
        project_id: None,
        archived: false,
        search_term,
        relation_filter: None,
        use_state_db_only: true,
    }
}
