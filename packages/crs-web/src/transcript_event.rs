use crate::Transcript;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum TranscriptEvent {
    #[serde(rename = "transcript.text.done")]
    Done(Transcript),
    #[serde(rename = "error")]
    Error,
    #[serde(other)]
    Other,
}
