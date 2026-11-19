use crate::PendingSteer;
use codex_app_server_protocol::{QueuedSubmission, UserInput};
use serde::Serialize;

#[derive(Clone, Hash, PartialEq, Serialize)]
pub struct QueuedMessage {
    pub id: String,
    pub client_id: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit_texts: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    pub confirmed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery: Option<String>,
}

impl From<&QueuedSubmission> for QueuedMessage {
    fn from(item: &QueuedSubmission) -> Self {
        use UserInput::*;
        let simple = matches!(item.input.as_slice(), [Text { text_elements, .. }] if text_elements.is_empty());
        let text = item
            .input
            .iter()
            .map(|input| match input {
                Text {
                    text,
                    ..
                } => text.as_str(),
                Image {
                    ..
                }
                | LocalImage {
                    ..
                } => "[Image]",
                Audio {
                    ..
                }
                | LocalAudio {
                    ..
                } => "[Audio]",
                Skill {
                    name,
                    ..
                }
                | Mention {
                    name,
                    ..
                } => name.as_str(),
            })
            .collect::<Vec<_>>()
            .join("\n");
        Self {
            id: item.id.clone(),
            client_id: item.client_user_message_id.clone(),
            text,
            edit_texts: (!simple).then(|| edit_texts(&item.input).map(str::to_owned).collect()),
            turn_id: None,
            confirmed: false,
            recovery: None,
        }
    }
}

impl TryFrom<&PendingSteer> for QueuedMessage {
    type Error = serde_json::Error;

    fn try_from(steer: &PendingSteer) -> Result<Self, Self::Error> {
        let mut preview = Self::from(&steer.item);
        preview.recovery = Some(serde_json::to_string_pretty(&steer.item)?);
        preview.turn_id = Some(steer.turn_id.clone());
        preview.confirmed = steer.confirmed;
        Ok(preview)
    }
}

pub fn edit_texts(input: &[UserInput]) -> impl Iterator<Item = &str> {
    input.iter().filter_map(|part| match part {
        UserInput::Text {
            text,
            text_elements,
        } if text_elements.is_empty() => Some(text.as_str()),
        _ => None,
    })
}
