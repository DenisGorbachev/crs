use codex_app_server_protocol::SortDirection::*;
use codex_app_server_protocol::ThreadSortKey::*;
use codex_app_server_protocol::ThreadSourceKind::*;
use codex_app_server_protocol::{ThreadListCwdFilter, ThreadListParams};

pub fn list_threads_params_all_reverse(cwd_filters: Vec<String>, search_term: Option<String>) -> ThreadListParams {
    ThreadListParams {
        limit: Some(u32::MAX),
        cursor: None,
        sort_key: Some(CreatedAt),
        sort_direction: Some(Desc),
        source_kinds: Some(vec![Cli, VsCode, Exec, AppServer, SubAgent, Unknown]),
        model_providers: Some(Vec::new()),
        cwd: (!cwd_filters.is_empty()).then_some(ThreadListCwdFilter::Many(cwd_filters)),
        section_id: None,
        project_id: None,
        archived: Some(false),
        search_term,
        parent_thread_id: None,
        ancestor_thread_id: None,
        use_state_db_only: true,
    }
}
