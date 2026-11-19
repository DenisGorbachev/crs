use crate::{FileLinks, code_ranges};
use dioxus::prelude::*;
use pulldown_cmark::html::push_html;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::iter::from_fn;
use url::{ParseError, Url};

pub fn thread_markdown(markdown: &str) -> Parser<'_> {
    Parser::new_ext(markdown, Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS)
}

pub fn markdown_view(markdown: &str, files: &FileLinks<'_>) -> Element {
    let mut events = thread_markdown(markdown);
    let mut html = String::new();
    let safe_events = from_fn(|| {
        Some(match events.next()? {
            Event::Start(Tag::CodeBlock(kind)) => {
                let mut source = String::new();
                for event in events.by_ref() {
                    match event {
                        Event::End(TagEnd::CodeBlock) => break,
                        Event::Text(text) => source.push_str(&text),
                        _ => (),
                    }
                }
                Event::Html(code_html(&kind, source).into())
            }
            Event::Start(Tag::Link {
                dest_url,
                ..
            }) if dest_url.starts_with("~/") => Event::Html(r#"<a aria-disabled="true" title="Use an absolute path. The Codex server's home directory is unknown.">"#.into()),
            Event::Start(mut tag) => {
                let file_href = files.href(&tag);
                if let Tag::Link {
                    dest_url,
                    ..
                }
                | Tag::Image {
                    dest_url,
                    ..
                } = &mut tag
                {
                    if let Some(href) = file_href {
                        *dest_url = href.into();
                    } else {
                        match Url::parse(dest_url) {
                            Ok(url) if matches!(url.scheme(), "http" | "https" | "mailto") => (),
                            Err(ParseError::RelativeUrlWithoutBase) => (),
                            _ => *dest_url = "#".into(),
                        }
                    }
                }
                Event::Start(tag)
            }
            Event::Html(text) | Event::InlineHtml(text) => Event::Text(text),
            Event::HardBreak => Event::Html("<br>".into()),
            event => event,
        })
    });
    push_html(&mut html, safe_events);
    rsx! { div { class: "markdown", dangerous_inner_html: html } }
}

fn code_html(kind: &CodeBlockKind<'_>, source: String) -> String {
    let language = match kind {
        CodeBlockKind::Fenced(info) => info.split_whitespace().next().filter(|word| {
            word.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"+.-_".contains(&b))
        }),
        CodeBlockKind::Indented => None,
    }
    .unwrap_or_default();
    let ranges = code_ranges(&source, language);
    let mut html = String::new();
    let open = format!("<pre><code data-language=\"{language}\" data-ranges=\"{ranges}\">");
    push_html(
        &mut html,
        [
            Event::Html(open.into()),
            Event::Text(source.into()),
            Event::Html("</code></pre>\n".into()),
        ]
        .into_iter(),
    );
    html
}
