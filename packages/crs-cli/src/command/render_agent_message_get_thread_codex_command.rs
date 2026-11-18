use crate::{CodexThreadId, MARKDOWN_SEPARATOR, PageStreamError, list_items_params_all_reverse, page_stream};
use clap::Parser;
use codex_app_server_client::{RemoteAppServerClient, TypedRequestError};
use codex_app_server_protocol::ThreadItemPayload::*;
use codex_app_server_protocol::{ClientRequest, RequestId, ThreadItemsListParams, ThreadItemsListResponse};
use errgonomic::{handle, handle_bool};
use futures::TryStreamExt;
use futures::future::ready;
use futures::stream::iter;
use std::io::{self, Write, stdout};
use std::pin::pin;
use std::process::ExitCode;
use thiserror::Error;
use uuid::Uuid;

#[derive(Parser, Clone, Debug)]
#[command(flatten_help = true, about = "Print agent messages newest first, separated by Markdown horizontal rules.")]
pub struct RenderAgentMessageGetThreadCodexCommand {
    /// Zero-based index of the first agent message to print.
    #[arg(long, default_value_t = 0)]
    pub index: usize,
    #[arg(long, default_value_t = 1, conflicts_with = "all", value_name = "COUNT", help = "Maximum number of agent messages to print.")]
    pub limit: usize,
    #[arg(long, help = "Print all remaining agent messages.")]
    pub all: bool,
}

impl RenderAgentMessageGetThreadCodexCommand {
    pub async fn run(self, client: &mut RemoteAppServerClient, thread_id: CodexThreadId) -> Result<ExitCode, RenderAgentMessageGetThreadCodexCommandRunError> {
        use RenderAgentMessageGetThreadCodexCommandRunError::*;
        let Self {
            index,
            limit,
            all,
        } = self;
        let params = list_items_params_all_reverse(thread_id);
        let mut skipped = 0..index;
        let messages = page_stream(client, params, |client, params| {
            client.request_typed::<ThreadItemsListResponse>(ClientRequest::ThreadItemsList {
                request_id: RequestId::String(Uuid::new_v4().to_string()),
                params,
            })
        })
        .map_ok(|items| {
            iter(
                items
                    .into_iter()
                    .filter_map(|entry| match entry.item.payload {
                        AgentMessage {
                            text,
                            ..
                        } => Some(Ok::<_, PageStreamError<ThreadItemsListParams, TypedRequestError>>(text)),
                        _ => None,
                    }),
            )
        })
        .try_flatten()
        .try_skip_while(move |_| ready(Ok(skipped.next().is_some())));
        let mut messages = pin!(messages);
        let mut quota = 0..limit;
        let mut wrote_message = false;
        while all || quota.next().is_some() {
            let Some(text) = handle!(messages.try_next().await, TryNextFailed, thread_id, index) else {
                handle_bool!(!wrote_message && (index != 0 || !all), AgentMessageNotFound, thread_id, index);
                break;
            };
            let mut output = stdout().lock();
            if wrote_message {
                handle!(output.write_all(MARKDOWN_SEPARATOR.as_bytes()), WriteSeparatorFailed, thread_id);
            }
            handle!(output.write_all(text.as_bytes()), WriteAllFailed, text);
            wrote_message = true;
        }
        handle!(stdout().flush(), FlushFailed, thread_id);
        Ok(ExitCode::SUCCESS)
    }
}

#[derive(Error, Debug)]
pub enum RenderAgentMessageGetThreadCodexCommandRunError {
    #[error("failed to read agent messages starting at index {index} in Codex thread '{thread_id}'")]
    TryNextFailed { source: PageStreamError<ThreadItemsListParams, TypedRequestError>, thread_id: CodexThreadId, index: usize },
    #[error("agent message at index {index} was not found in Codex thread '{thread_id}'")]
    AgentMessageNotFound { thread_id: CodexThreadId, index: usize },
    #[error("failed to write the message separator for Codex thread '{thread_id}'")]
    WriteSeparatorFailed { source: io::Error, thread_id: CodexThreadId },
    #[error("failed to write the agent message to stdout")]
    WriteAllFailed { source: io::Error, text: String },
    #[error("failed to flush agent messages from Codex thread '{thread_id}' to stdout")]
    FlushFailed { source: io::Error, thread_id: CodexThreadId },
}
