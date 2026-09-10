use crate::{ListThreadsParams, PageStreamError, WriteJsonlError, list_threads_params_all_reverse, page_stream, write_jsonl};
use clap::Parser;
use codex_thread_store::{StoredThread, ThreadStore, ThreadStoreError};
use errgonomic::handle;
use futures::future::ready;
use futures::stream::iter;
use futures::{StreamExt, TryStreamExt};
use std::io::{self, Write, stdout};
use std::path::PathBuf;
use std::process::ExitCode;
use thiserror::Error;

/// Filter Codex threads, emitting JSONL newest first.
#[derive(Parser, Clone, Debug)]
#[command(flatten_help = true)]
pub struct FilterThreadCodexCommand {
    /// Search the thread title or preview.
    #[arg(long)]
    pub search_term: Option<String>,
    /// Restrict threads to these working directories; omit to search all directories.
    #[arg(long, num_args = 1.., value_name = "PATH")]
    pub cwd_filters: Option<Vec<PathBuf>>,
    /// Number of matching threads to skip, starting with the newest.
    #[arg(long, default_value_t = 0)]
    pub offset: usize,
    /// Maximum number of threads to emit.
    #[arg(long, default_value_t = 10)]
    pub limit: usize,
}

impl FilterThreadCodexCommand {
    pub async fn run(self, store: &(impl ThreadStore + ?Sized)) -> Result<ExitCode, FilterThreadCodexCommandRunError> {
        use FilterThreadCodexCommandRunError::*;
        let Self {
            search_term,
            cwd_filters,
            offset,
            limit,
        } = self;
        let mut params = list_threads_params_all_reverse(cwd_filters, search_term);
        params.page_size = offset
            .checked_add(limit)
            .unwrap_or(params.page_size)
            .min(params.page_size);
        let mut skipped = 0..offset;
        page_stream(store, params, ThreadStore::list_threads)
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
    PageStreamFailed { source: PageStreamError<ListThreadsParams, ThreadStoreError> },
    #[error("failed to write a Codex thread to stdout")]
    WriteJsonlFailed { source: WriteJsonlError<StoredThread> },
    #[error("failed to flush Codex threads to stdout")]
    FlushFailed { source: io::Error },
}
