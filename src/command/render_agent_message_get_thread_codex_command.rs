use crate::{CodexThreadId, PageStreamError, list_items_params_all_reverse};
use clap::Parser;
use codex_app_server_client::{RemoteAppServerClient, TypedRequestError};
use codex_app_server_protocol::ThreadItem::*;
use codex_app_server_protocol::{ThreadItemsListParams, ThreadItemsListResponse};
use errgonomic::{handle, handle_opt};
use futures::TryStreamExt;
use futures::future::ready;
use futures::stream::iter;
use std::io::{self, Write, stdout};
use std::pin::pin;
use std::process::ExitCode;
use thiserror::Error;

#[derive(Parser, Clone, Debug)]
#[command(flatten_help = true)]
pub struct RenderAgentMessageGetThreadCodexCommand {
    /// Zero-based agent message index, starting with the newest message.
    #[arg(long, default_value_t = 0)]
    pub index: usize,
}

impl RenderAgentMessageGetThreadCodexCommand {
    pub async fn run(self, client: &mut RemoteAppServerClient, thread_id: CodexThreadId) -> Result<ExitCode, RenderAgentMessageGetThreadCodexCommandRunError> {
        use RenderAgentMessageGetThreadCodexCommandRunError::*;
        let Self {
            index,
        } = self;
        let params = list_items_params_all_reverse(thread_id);
        let mut skipped = 0..index;
        let messages = app_server_pages!(client, params, ThreadItemsList, ThreadItemsListResponse)
            .map_ok(|items| {
                iter(items.into_iter().filter_map(|entry| match entry.item {
                    AgentMessage {
                        text,
                        ..
                    } => Some(Ok::<_, PageStreamError<ThreadItemsListParams, TypedRequestError>>(text)),
                    _ => None,
                }))
            })
            .try_flatten()
            .try_skip_while(move |_| ready(Ok(skipped.next().is_some())));
        let text = handle!(pin!(messages).try_next().await, TryNextFailed, thread_id, index);
        let text = handle_opt!(text, AgentMessageNotFound, thread_id, index);
        let mut stdout = stdout().lock();
        handle!(stdout.write_all(text.as_bytes()), WriteAllFailed, text);
        handle!(stdout.flush(), FlushFailed, text);
        Ok(ExitCode::SUCCESS)
    }
}

#[derive(Error, Debug)]
pub enum RenderAgentMessageGetThreadCodexCommandRunError {
    #[error("failed to select agent message at index {index} in Codex thread '{thread_id}'")]
    TryNextFailed { source: PageStreamError<ThreadItemsListParams, TypedRequestError>, thread_id: CodexThreadId, index: usize },
    #[error("agent message at index {index} was not found in Codex thread '{thread_id}'")]
    AgentMessageNotFound { thread_id: CodexThreadId, index: usize },
    #[error("failed to write the agent message to stdout")]
    WriteAllFailed { source: io::Error, text: String },
    #[error("failed to flush the agent message to stdout")]
    FlushFailed { source: io::Error, text: String },
}
