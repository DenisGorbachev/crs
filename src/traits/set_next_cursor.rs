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

mod thread_list_params;
pub use thread_list_params::*;
mod thread_items_list_params;
pub use thread_items_list_params::*;
