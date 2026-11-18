use crate::CodexConnectionRequestError;
use errgonomic::{handle, handle_bool};
use std::collections::HashSet;
use thiserror::Error;

pub async fn fetch_pages<T, Fut>(mut fetch: impl FnMut(Option<String>) -> Fut) -> Result<Vec<T>, FetchPagesError>
where
    Fut: Future<Output = Result<(Vec<T>, Option<String>), CodexConnectionRequestError>> + Send,
{
    use FetchPagesError::*;
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    let mut cursor = None;
    loop {
        let (page, next_cursor) = handle!(fetch(cursor).await, FetchFailed);
        items.extend(page);
        let Some(next_cursor) = next_cursor else {
            return Ok(items);
        };
        handle_bool!(!seen.insert(next_cursor.clone()), CursorInvalid);
        cursor = Some(next_cursor);
    }
}

#[derive(Error, Debug)]
pub enum FetchPagesError {
    #[error("failed to fetch a Codex history page")]
    FetchFailed { source: CodexConnectionRequestError },
    #[error("Codex returned a repeated history cursor")]
    CursorInvalid {},
}
