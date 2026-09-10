/// Consumes a page into its items and the cursor for the next request.
pub trait IntoItemsAndCursor {
    type Items;
    type Cursor;

    /// A missing cursor marks the last page, even when its items are empty.
    fn into_items_and_cursor(self) -> (Self::Items, Option<Self::Cursor>);
}

macro_rules! impl_into_items_and_cursor {
    ($page:ty, $items:ty, $cursor:ty) => {
        impl IntoItemsAndCursor for $page {
            type Items = $items;
            type Cursor = $cursor;

            fn into_items_and_cursor(self) -> (Self::Items, Option<Self::Cursor>) {
                (self.items, self.next_cursor)
            }
        }
    };
}

mod thread_page;
pub use thread_page::*;
mod item_page;
pub use item_page::*;
