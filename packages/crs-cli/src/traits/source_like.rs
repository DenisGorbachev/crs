use crate::ReviewItemLike;
use fjall::SingleWriterWriteTx;
use futures::Stream;
use std::error::Error;

pub trait SourceLike {
    type ReviewItem: ReviewItemLike;
    type ReviewItemError: Error;
    type ReviewItemsError: Error;

    async fn review_items(&self, tx: &SingleWriterWriteTx) -> Result<impl Stream<Item = Result<Self::ReviewItem, Self::ReviewItemError>>, Self::ReviewItemsError>;
}
