use errgonomic::handle_bool;
use std::fmt;
use std::str::FromStr;
use thiserror::Error;
use url::form_urlencoded::byte_serialize;

#[derive(Clone, Debug, PartialEq)]
pub struct UrlSegment {
    pub value: String,
}

impl fmt::Display for UrlSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if matches!(self.value.as_str(), "." | "..") || self.value.starts_with('~') {
            f.write_str("~")?;
        }
        // Dioxus decodes each path segment after splitting it, but leaves slashes and percent signs unescaped when writing one.
        byte_serialize(self.value.as_bytes()).try_for_each(|part| f.write_str(if part == "+" { "%20" } else { part }))
    }
}

impl FromStr for UrlSegment {
    type Err = UrlSegmentFromStrError;

    fn from_str(segment: &str) -> Result<Self, Self::Err> {
        use UrlSegmentFromStrError::*;
        let name = segment
            .strip_prefix('~')
            .filter(|name| matches!(*name, "." | "..") || name.starts_with('~'))
            .unwrap_or(segment);
        handle_bool!(name.is_empty() || name.contains('\0'), SegmentInvalid);
        Ok(Self {
            value: name.into(),
        })
    }
}

#[derive(Error, Debug, Clone, Copy, PartialEq)]
pub enum UrlSegmentFromStrError {
    #[error("URL segment must be nonempty and contain no NUL bytes")]
    SegmentInvalid {},
}
