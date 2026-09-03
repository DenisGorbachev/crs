use crate::MarkdownParser;
use pulldown_cmark::Options;

pub fn markdown_parser_new(document: &str) -> MarkdownParser<'_> {
    MarkdownParser::new_ext(document, Options::all())
}
