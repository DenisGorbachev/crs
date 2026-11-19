use crate::{SharedServer, TranscriptEvent};
use axum::body::Bytes;
use axum::extract::{FromRequest, Request};
use axum::http::{StatusCode, header::CONTENT_TYPE};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use errgonomic::handle;
use eventsource_stream::{EventStreamError, Eventsource};
use futures::{StreamExt, stream};
use openai_dive::v1::api::Client;
use openai_dive::v1::error::APIError;
use openai_dive::v1::resources::shared::FileUploadBytes;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use thiserror::Error;

#[derive(Deserialize, Serialize)]
pub struct Transcript {
    pub text: String,
}

impl Transcript {
    pub async fn decode(body: &str, streamed: bool) -> Result<Self, TranscriptDecodeError> {
        use TranscriptDecodeError::*;
        if !streamed {
            return Ok(handle!(serde_json::from_str(body), FromJsonFailed));
        }
        let mut events = stream::iter(body.as_bytes().chunks(4096).map(Ok::<_, Infallible>)).eventsource();
        while let Some(event) = events.next().await {
            let event = handle!(event, ReadEventFailed);
            if event.data == "[DONE]" {
                break;
            }
            match handle!(serde_json::from_str(&event.data), DecodeEventFailed) {
                TranscriptEvent::Done(transcript) => return Ok(transcript),
                TranscriptEvent::Error => return Err(TranscriptionFailed {}),
                TranscriptEvent::Other => (),
            }
        }
        Err(FinalTranscriptNotFound {})
    }
}

#[derive(Error, Debug)]
pub enum TranscriptDecodeError {
    #[error("invalid transcription JSON")]
    FromJsonFailed { source: serde_json::Error },
    #[error("invalid transcription event stream")]
    ReadEventFailed { source: EventStreamError<Infallible> },
    #[error("invalid transcription event")]
    DecodeEventFailed { source: serde_json::Error },
    #[error("transcription stream reported an error")]
    TranscriptionFailed {},
    #[error("transcription stream ended before completion")]
    FinalTranscriptNotFound {},
}

pub async fn transcribe(Extension(server): Extension<SharedServer>, Extension(client): Extension<Option<Arc<Client>>>, request: Request) -> Response {
    let config = {
        let server = server.lock().await;
        if !request
            .headers()
            .get("x-crs-form-key")
            .is_some_and(|key| key == server.form_key.to_string().as_str())
        {
            return (StatusCode::FORBIDDEN, "Reload the form to refresh its key.").into_response();
        }
        server.config.clone()
    };
    let Some(client) = client else {
        return (StatusCode::SERVICE_UNAVAILABLE, "Configure OPENAI_API_KEY on the CRS-Web server to transcribe audio.").into_response();
    };
    if !request
        .headers()
        .get(CONTENT_TYPE)
        .is_some_and(|mime| mime == "audio/wav")
    {
        return (StatusCode::UNSUPPORTED_MEDIA_TYPE, "Expected WAV audio.").into_response();
    }
    let bytes = match Bytes::from_request(request, &()).await {
        Ok(bytes) => bytes,
        Err(error) => return error.into_response(),
    };
    if !bytes.starts_with(b"RIFF") || bytes.get(8..12) != Some(b"WAVE") || bytes.len() <= 44 {
        return (StatusCode::BAD_REQUEST, "The recording contains no usable WAV audio.").into_response();
    }
    let params = config
        .transcribe_params
        .request(FileUploadBytes::new(bytes, "recording.wav"));
    match client.audio().create_transcription(params).await {
        Ok(body) => match Transcript::decode(&body, config.transcribe_params.stream == Some(true)).await {
            Ok(transcript) if !transcript.text.trim().is_empty() => Json(transcript).into_response(),
            Ok(_) => (StatusCode::UNPROCESSABLE_ENTITY, "No speech was transcribed.").into_response(),
            Err(_) => (StatusCode::BAD_GATEWAY, "OpenAI returned an unreadable transcript.").into_response(),
        },
        Err(error) => {
            use APIError::*;
            let message = match error {
                AuthenticationError(_) | PermissionError(_) => "Check the CRS-Web server’s OpenAI key and model access.",
                RateLimitError(_) => "OpenAI rate limit reached. Retry later.",
                BadRequestError(_) | InvalidRequestError(_) => "OpenAI rejected the recording or transcription settings.",
                _ => "OpenAI transcription failed. Retry the recording.",
            };
            (StatusCode::BAD_GATEWAY, message).into_response()
        }
    }
}
