use crate::{IntoItemsAndCursor, SetNextCursor};
use errgonomic::handle;
use futures::Stream;
use futures::stream::try_unfold;
use std::error::Error as StdError;
use thiserror::Error;

/// Lazily fetches pages, yielding their items without flattening and stopping after an error.
pub fn page_stream<'a, S, P, R, E, Fut>(store: &'a S, params: P, fetch_page: fn(&'a S, P) -> Fut) -> impl Stream<Item = Result<R::Items, PageStreamError<P, E>>> + Send + 'a
where
    S: Sync + ?Sized,
    P: SetNextCursor + Clone + Send + 'a,
    R: IntoItemsAndCursor<Cursor = P::Cursor>,
    E: StdError + 'static,
    Fut: Future<Output = Result<R, E>> + Send + 'a,
{
    try_unfold(Some(params), move |next_params| async move {
        use PageStreamError::*;

        let Some(mut params) = next_params else {
            return Ok(None);
        };

        // The fetch function consumes its request; retain params for continuation and error context.
        let result = fetch_page(store, params.clone()).await;
        let page = handle!(result, FetchPageFailed, params);

        let (items, next_cursor) = page.into_items_and_cursor();
        let next_params = next_cursor.map(|cursor| {
            params.set_next_cursor(cursor);
            params
        });

        Ok(Some((items, next_params)))
    })
}

#[derive(Error, Debug)]
pub enum PageStreamError<P, E> {
    #[error("failed to fetch a page")]
    FetchPageFailed { source: E, params: Box<P> },
}
