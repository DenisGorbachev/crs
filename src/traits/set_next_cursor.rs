/// Updates a request with a continuation cursor while preserving its other fields.
pub trait SetNextCursor {
    type Cursor;

    fn set_next_cursor(&mut self, cursor: Self::Cursor);
}

macro_rules! impl_set_next_cursor {
    ($params:ty, $cursor:ty) => {
        impl SetNextCursor for $params {
            type Cursor = $cursor;

            fn set_next_cursor(&mut self, cursor: Self::Cursor) {
                self.cursor = Some(cursor);
            }
        }
    };
}

mod list_threads_params;
pub use list_threads_params::*;
mod list_items_params;
pub use list_items_params::*;
