use crate::{Route, UrlSegment, thread_markdown};
use codex_app_server_protocol::{Thread, ThreadItemPayload, Turn, UserInput};
use codex_utils_absolute_path::AbsolutePathBuf;
use pulldown_cmark::{Event, LinkType, Tag};
use url::{Url, form_urlencoded::Serializer};

pub struct FileLinks<'a> {
    pub route: String,
    thread: &'a str,
    directory: Option<Url>,
}

impl<'a> From<(&str, &'a Thread)> for FileLinks<'a> {
    fn from((source, thread): (&str, &'a Thread)) -> Self {
        let server = Route::Server {
            source: UrlSegment {
                value: source.into(),
            },
        };
        Self {
            route: format!("{server}/files"),
            thread: &thread.id,
            directory: Url::from_directory_path(&thread.cwd).ok(),
        }
    }
}

impl FileLinks<'_> {
    fn path(&self, tag: &Tag<'_>) -> Option<AbsolutePathBuf> {
        let Tag::Link {
            link_type,
            dest_url,
            ..
        } = tag
        else {
            return None;
        };
        let destination = dest_url.as_ref();
        if *link_type == LinkType::Email || destination.is_empty() || destination.starts_with(['#', '?']) || destination.starts_with("//") || destination.starts_with("~/") {
            return None;
        }
        let original = destination.split('#').next()?;
        let mut path = original;
        while let Some((prefix, position)) = path.rsplit_once(':') {
            if !position
                .split(['-', '–'])
                .all(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))
            {
                break;
            }
            path = prefix;
        }
        if path != original && Url::parse(original).is_ok_and(|url| matches!(url.scheme(), "http" | "https" | "mailto" | "data" | "javascript" | "tel")) {
            return None;
        }
        let url = match path.as_bytes() {
            [drive, b':', b'/' | b'\\', ..] if cfg!(windows) && drive.is_ascii_alphabetic() => Url::parse(&format!("file:///{path}")),
            [b'\\', b'\\', ..] if cfg!(windows) => Url::parse(&format!("file:{path}")),
            [b'\\', ..] => return None,
            _ => Url::options().base_url(self.directory.as_ref()).parse(path),
        }
        .ok()?;
        if url.scheme() != "file" {
            return None;
        }
        AbsolutePathBuf::from_absolute_path_checked(url.to_file_path().ok()?).ok()
    }

    pub fn href(&self, tag: &Tag<'_>) -> Option<String> {
        let path = self.path(tag)?;
        let query = Serializer::new(String::new())
            .append_pair("thread", self.thread)
            .append_pair("path", path.to_str()?)
            .finish();
        Some(format!("{}?{query}", self.route))
    }

    pub fn contains(&self, turns: &[Turn], path: &AbsolutePathBuf) -> bool {
        let linked = |text: &str| thread_markdown(text).any(|event| matches!(event, Event::Start(tag) if self.path(&tag).as_ref() == Some(path)));
        turns
            .iter()
            .flat_map(|turn| &turn.items)
            .any(|item| match &item.payload {
                ThreadItemPayload::AgentMessage {
                    text,
                    ..
                }
                | ThreadItemPayload::Plan {
                    text,
                    ..
                } => linked(text),
                ThreadItemPayload::UserMessage {
                    content,
                    ..
                } => content
                    .iter()
                    .any(|input| matches!(input, UserInput::Text { text, .. } if linked(text))),
                _ => false,
            })
    }
}
