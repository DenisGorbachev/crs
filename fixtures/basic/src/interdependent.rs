/// Returns whether `value` is even.
pub fn is_even(value: u8) -> bool {
    value.checked_sub(1).is_none_or(is_odd)
}

/// Returns whether `value` is odd.
pub fn is_odd(value: u8) -> bool {
    value.checked_sub(1).is_some_and(is_even)
}
