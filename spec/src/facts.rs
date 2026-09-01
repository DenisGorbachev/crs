pub fn rust_code_items_may_be_interdependent() -> bool {
    !interdependent_code_items_examples().is_empty()
}

pub fn interdependent_code_items_examples() -> &'static [&'static str] {
    &[include_str!("../../fixtures/basic/src/interdependent.rs")]
}
