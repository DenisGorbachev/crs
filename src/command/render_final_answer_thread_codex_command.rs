use crate::CodexThreadId;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::ThreadItem::*;
use codex_core::config::Config;
use codex_protocol::models::MessagePhase::*;
use codex_rollout::state_db::get_state_db;
use codex_thread_store::ItemSortKey::*;
use codex_thread_store::SortDirection::*;
use codex_thread_store::{ListItemsParams, LocalThreadStore, LocalThreadStoreConfig, StoredThreadItem, ThreadStoreError};
use errgonomic::{handle, handle_opt};
use std::io::{self, Write, stdout};
use std::process::ExitCode;
use thiserror::Error;

#[derive(clap::Parser, Clone, Debug)]
#[command(flatten_help = true)]
pub struct RenderFinalAnswerThreadCodexCommand {}

impl RenderFinalAnswerThreadCodexCommand {
    pub async fn run(self, thread_id: CodexThreadId) -> Result<ExitCode, RenderFinalAnswerThreadCodexCommandRunError> {
        use RenderFinalAnswerThreadCodexCommandRunError::*;
        let Self {} = self;
        let config = handle!(Config::load_with_cli_overrides(Vec::new()).await, LoadWithCliOverridesFailed);
        let state_db = get_state_db(&config).await;
        let store = LocalThreadStore::new(LocalThreadStoreConfig::from_config(&config), state_db);
        let page = handle!(
            store
                .list_items(ListItemsParams {
                    thread_id,
                    turn_id: None,
                    include_archived: true,
                    cursor: None,
                    page_size: 1,
                    sort_direction: Desc,
                    sort_key: CreatedAtOrdinal,
                    after_updated_at_ordinal: None,
                })
                .await,
            ListItemsFailed,
            thread_id
        );
        let stored_item = handle_opt!(page.items.into_iter().next(), ThreadItemNotFound, thread_id);
        let StoredThreadItem {
            item_json,
            ..
        } = stored_item;
        let item = handle!(serde_json::from_slice(&item_json), FromSliceFailed, item_json);
        let text = match item {
            AgentMessage {
                text,
                phase: Some(FinalAnswer),
                ..
            } => text,
            item => {
                return Err(ThreadItemInvalid {
                    thread_id,
                    item: Box::new(item),
                });
            }
        };
        let mut stdout = stdout().lock();
        handle!(stdout.write_all(text.as_bytes()), WriteAllFailed, text);
        handle!(stdout.flush(), FlushFailed, text);
        Ok(ExitCode::SUCCESS)
    }
}

#[derive(Error, Debug)]
pub enum RenderFinalAnswerThreadCodexCommandRunError {
    #[error("failed to load the Codex configuration")]
    LoadWithCliOverridesFailed { source: io::Error },
    #[error("failed to list items in Codex thread '{thread_id}'")]
    ListItemsFailed { source: ThreadStoreError, thread_id: CodexThreadId },
    #[error("no items were found in Codex thread '{thread_id}'")]
    ThreadItemNotFound { thread_id: CodexThreadId },
    #[error("failed to deserialize a projected Codex thread item")]
    FromSliceFailed { source: serde_json::Error, item_json: Vec<u8> },
    #[error("the last item in Codex thread '{thread_id}' is not a final answer")]
    ThreadItemInvalid { thread_id: CodexThreadId, item: Box<ThreadItem> },
    #[error("failed to write the final answer to stdout")]
    WriteAllFailed { source: io::Error, text: String },
    #[error("failed to flush the final answer to stdout")]
    FlushFailed { source: io::Error, text: String },
}
