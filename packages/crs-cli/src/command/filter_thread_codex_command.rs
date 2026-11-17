use crate::{PageStreamError, WriteJsonlError, list_threads_params_all_reverse, page_stream, write_jsonl};
use clap::Parser;
use codex_app_server_client::{RemoteAppServerClient, TypedRequestError};
use codex_app_server_protocol::{ClientRequest, RequestId, Thread, ThreadListParams, ThreadListResponse};
use errgonomic::handle;
use futures::future::ready;
use futures::stream::iter;
use futures::{StreamExt, TryStreamExt};
use std::io::{self, Write, stdout};
use std::process::ExitCode;
use thiserror::Error;
use uuid::Uuid;

/// Filter Codex threads, emitting JSONL newest first.
#[derive(Parser, Clone, Debug)]
#[command(flatten_help = true)]
pub struct FilterThreadCodexCommand {
    /// Search the thread title or preview.
    #[arg(long)]
    pub search_term: Option<String>,
    /// Restrict threads to these working directories; omit to search all directories.
    #[arg(long, num_args = 1.., value_name = "PATH")]
    pub cwd_filters: Vec<String>,
    /// Number of matching threads to skip, starting with the newest.
    #[arg(long, default_value_t = 0)]
    pub offset: usize,
    /// Maximum number of threads to emit.
    #[arg(long, default_value_t = 10)]
    pub limit: usize,
}

impl FilterThreadCodexCommand {
    pub async fn run(self, client: &mut RemoteAppServerClient) -> Result<ExitCode, FilterThreadCodexCommandRunError> {
        use FilterThreadCodexCommandRunError::*;
        let Self {
            search_term,
            cwd_filters,
            offset,
            limit,
        } = self;
        let mut params = list_threads_params_all_reverse(cwd_filters, search_term);
        params.limit = Some(match offset.checked_add(limit) {
            Some(count) => u32::try_from(count).unwrap_or(u32::MAX),
            None => u32::MAX,
        });
        let mut skipped = 0..offset;
        page_stream(client, params, |client, params| {
            client.request_typed::<ThreadListResponse>(ClientRequest::ThreadList {
                request_id: RequestId::String(Uuid::new_v4().to_string()),
                params,
            })
        })
        .map(|result| {
            let threads = handle!(result, PageStreamFailed);
            Ok(iter(threads.into_iter().map(Ok)))
        })
        .try_flatten()
        .try_skip_while(move |_| ready(Ok(skipped.next().is_some())))
        .take(limit)
        .try_for_each(|thread| async move {
            let mut stdout = stdout().lock();
            handle!(write_jsonl(&mut stdout, thread), WriteJsonlFailed);
            Ok(())
        })
        .await
        .and_then(|()| {
            handle!(stdout().flush(), FlushFailed);
            Ok(ExitCode::SUCCESS)
        })
    }
}

#[derive(Error, Debug)]
pub enum FilterThreadCodexCommandRunError {
    #[error("failed to read Codex threads")]
    PageStreamFailed { source: PageStreamError<ThreadListParams, TypedRequestError> },
    #[error("failed to write a Codex thread to stdout")]
    WriteJsonlFailed { source: WriteJsonlError<Thread> },
    #[error("failed to flush Codex threads to stdout")]
    FlushFailed { source: io::Error },
}
