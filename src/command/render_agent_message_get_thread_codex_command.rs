use crate::{CodexThreadId, list_items_params_all_reverse};
use clap::Parser;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::ThreadItem::*;
use codex_thread_store::{StoredThreadItem, ThreadStore, ThreadStoreError};
use errgonomic::{handle, handle_opt};
use itertools::process_results;
use std::io::{self, Write, stdout};
use std::process::ExitCode;
use thiserror::Error;

#[derive(Parser, Clone, Debug)]
#[command(flatten_help = true)]
pub struct RenderAgentMessageGetThreadCodexCommand {
    /// Zero-based agent message index within the first page, starting with the newest message.
    #[arg(long, default_value_t = 0)]
    pub index: usize,
}

impl RenderAgentMessageGetThreadCodexCommand {
    pub async fn run(self, store: &(impl ThreadStore + ?Sized), thread_id: CodexThreadId) -> Result<ExitCode, RenderAgentMessageGetThreadCodexCommandRunError> {
        use RenderAgentMessageGetThreadCodexCommandRunError::*;
        let Self {
            index,
        } = self;
        let params = list_items_params_all_reverse(thread_id);
        let page = handle!(store.list_items(params).await, ListItemsFailed, thread_id, index);
        let items = page.items.into_iter().map(
            |StoredThreadItem {
                 item_json,
                 ..
             }| { Ok(handle!(serde_json::from_slice::<ThreadItem>(&item_json), FromSliceFailed, thread_id, index, item_json)) },
        );
        process_results(items, |items| {
            items
                .filter_map(|item| match item {
                    AgentMessage {
                        text,
                        ..
                    } => Some(text),
                    _ => None,
                })
                .nth(index)
        })
        .and_then(|text| {
            let text = handle_opt!(text, AgentMessageNotFound, thread_id, index);
            let mut stdout = stdout().lock();
            handle!(stdout.write_all(text.as_bytes()), WriteAllFailed, text);
            handle!(stdout.flush(), FlushFailed, text);
            Ok(ExitCode::SUCCESS)
        })
    }
}

#[derive(Error, Debug)]
pub enum RenderAgentMessageGetThreadCodexCommandRunError {
    #[error("failed to list items in Codex thread '{thread_id}'")]
    ListItemsFailed { source: ThreadStoreError, thread_id: CodexThreadId, index: usize },
    #[error("agent message at index {index} was not found in Codex thread '{thread_id}'")]
    AgentMessageNotFound { thread_id: CodexThreadId, index: usize },
    #[error("failed to deserialize a projected item in Codex thread '{thread_id}' while selecting agent message at index {index}")]
    FromSliceFailed { source: serde_json::Error, thread_id: CodexThreadId, index: usize, item_json: Vec<u8> },
    #[error("failed to write the agent message to stdout")]
    WriteAllFailed { source: io::Error, text: String },
    #[error("failed to flush the agent message to stdout")]
    FlushFailed { source: io::Error, text: String },
}
