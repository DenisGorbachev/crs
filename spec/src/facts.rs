pub fn rust_items_may_be_interdependent() -> bool {
    !interdependent_rust_item_examples().is_empty()
}

pub fn interdependent_rust_item_examples() -> &'static [&'static str] {
    &[include_str!("../../fixtures/basic/src/interdependent.rs")]
}

pub fn rust_files_may_contain_meaningful_non_item_lines() -> bool {
    !meaningful_non_item_rust_lines().is_empty()
}

pub fn meaningful_non_item_rust_lines() -> &'static [&'static str] {
    &[
        "#![no_std]",
        "/// Doc comment",
        "// Regular single-line comment",
    ]
}

pub fn rust_analyzer_syntax_tree_retains_all_comments() -> bool {
    true
}
