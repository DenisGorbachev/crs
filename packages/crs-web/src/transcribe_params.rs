use openai_dive::v1::resources::audio::{AudioOutputFormat, AudioTranscriptionParameters, TranscriptionChunkingStrategy};
use openai_dive::v1::resources::shared::{FileUpload, FileUploadBytes};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TranscribeParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<AudioOutputFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunking_strategy: Option<TranscriptionChunkingStrategy>,
}

impl TranscribeParams {
    pub fn request(&self, audio: FileUploadBytes) -> AudioTranscriptionParameters {
        AudioTranscriptionParameters {
            file: FileUpload::Bytes(audio),
            model: "gpt-transcribe".into(),
            prompt: self.prompt.clone(),
            languages: Some(self.languages.clone()),
            keywords: Some(self.keywords.clone()),
            stream: self.stream,
            response_format: self.response_format.clone(),
            temperature: self.temperature,
            chunking_strategy: self.chunking_strategy.clone(),
            ..Default::default()
        }
    }
}
