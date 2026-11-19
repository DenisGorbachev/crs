use crate::{Route, UrlSegment};
use codex_utils_absolute_path::AbsolutePathBuf;
use pulldown_cmark::{LinkType, Tag};
use url::Url;

pub struct FileLinks {
    pub route: String,
    directory: Option<Url>,
}

impl From<(&str, &AbsolutePathBuf)> for FileLinks {
    fn from((source, cwd): (&str, &AbsolutePathBuf)) -> Self {
        let server = Route::Server {
            source: UrlSegment {
                value: source.into(),
            },
        };
        Self {
            route: format!("{server}/files"),
            directory: Url::from_directory_path(cwd).ok(),
        }
    }
}

impl FileLinks {
    pub fn href(&self, tag: &Tag<'_>) -> Option<String> {
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
        let path = AbsolutePathBuf::from_absolute_path_checked(url.to_file_path().ok()?).ok()?;
        let file = Url::from_file_path(path.to_str()?).ok()?;
        let path = file.path();
        Some(match file.host_str() {
            Some(host) => format!("{}//{host}{path}", self.route),
            None => format!("{}{path}", self.route),
        })
    }
}
