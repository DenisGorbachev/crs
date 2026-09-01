use crate::ReviewItemLike;
use futures::Stream;
use std::error::Error;

pub trait SourceLike {
    type ReviewItem: ReviewItemLike;
    type ReviewItemError: Error;
    type ReviewItemsError: Error;

    async fn review_items(&self) -> Result<impl Stream<Item = Result<Self::ReviewItem, Self::ReviewItemError>>, Self::ReviewItemsError>;
}
