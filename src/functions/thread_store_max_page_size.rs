/// The store fetches one extra row and requires that limit to fit in i64.
pub fn thread_store_max_page_size() -> usize {
    usize::try_from(i64::MAX - 1).unwrap_or(usize::MAX - 1)
}
