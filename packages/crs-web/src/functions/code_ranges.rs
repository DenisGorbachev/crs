use std::iter::successors;
use tree_sitter::{Node, Parser};

pub fn code_ranges(source: &str, language: &str) -> String {
    let grammar = match language.to_ascii_lowercase().as_str() {
        "rust" | "rs" => tree_sitter_rust::LANGUAGE,
        "python" | "py" => tree_sitter_python::LANGUAGE,
        "javascript" | "js" | "jsx" => tree_sitter_javascript::LANGUAGE,
        _ => return String::new(),
    };
    let mut parser = Parser::new();
    if parser.set_language(&grammar.into()).is_err() {
        return String::new();
    }
    let Some(tree) = parser.parse(source, None) else { return String::new() };
    let mut cursor = tree.walk();
    let mut ranges = String::new();
    loop {
        if cursor.goto_first_child() {
            continue;
        }
        let token = cursor.node();
        if let Some(target) = quote_target(token) {
            ranges.push_str(&format!("{}:{}:{}:{} ", token.start_byte(), token.end_byte(), target.start_byte(), target.end_byte()));
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return ranges;
            }
        }
    }
}

fn quote_target(token: Node<'_>) -> Option<Node<'_>> {
    if token.is_missing() || token.is_error() || token.start_byte() == token.end_byte() || token.parent().is_some_and(|parent| parent.is_error()) {
        return None;
    }
    let kinds: &[&str] = match token.kind() {
        "if" => &["if_expression", "if_statement"],
        "match" => &["match_expression", "match_statement"],
        "fn" | "def" | "function" => &[
            "function_item",
            "function_definition",
            "function_declaration",
            "function_expression",
            "generator_function",
            "generator_function_declaration",
        ],
        _ if token.is_named() => return Some(token),
        _ => {
            return token
                .parent()
                .filter(|node| node.is_named() && !node.has_error() && node.start_position().row == node.end_position().row);
        }
    };
    successors(token.parent(), Node::parent)
        .find(|node| kinds.contains(&node.kind()))
        .filter(|node| !node.has_error())
}
