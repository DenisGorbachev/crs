use crate::{ThreadRecord, canonical_json, content_version};
use serde::Serialize;
use serde_json::Error;

#[derive(Serialize)]
pub struct ThreadHeader {
    pub title: String,
    pub archived: bool,
    pub writable: bool,
    pub metadata: String,
    pub version: String,
}

impl TryFrom<&ThreadRecord> for ThreadHeader {
    type Error = Error;

    fn try_from(record: &ThreadRecord) -> Result<Self, Self::Error> {
        let mut json = canonical_json(record)?;
        if let Some(fields) = json.as_object_mut() {
            fields.remove("turn_versions");
        }
        let version = content_version(&json);
        Ok(Self {
            title: record.title().to_owned(),
            archived: record.archived,
            writable: record.thread.can_accept_direct_input != Some(false),
            metadata: serde_json::to_string_pretty(&json)?,
            version,
        })
    }
}
