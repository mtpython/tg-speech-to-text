pub mod elevenlabs;
pub mod whisper;
pub mod google;

use crate::{audio::ConvertedAudio, BotConfig};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SttError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),
    #[error("Authentication failed")]
    Authentication,
    #[error("Rate limit exceeded")]
    RateLimit,
    #[error("Service unavailable")]
    ServiceUnavailable,
}

#[derive(Debug, Clone, Copy)]
pub enum SttProvider {
    Whisper,
    ElevenLabs,
    Google,
}

pub async fn transcribe(audio: &ConvertedAudio, config: &BotConfig) -> Result<String, SttError> {
    match config.stt_provider {
        SttProvider::Whisper => {
            let api_key = config.openai_api_key.as_ref()
                .ok_or_else(|| SttError::Api("OpenAI API key not configured".to_string()))?;
            whisper::transcribe(audio, api_key).await
        }
        SttProvider::ElevenLabs => {
            let api_key = config.elevenlabs_api_key.as_ref()
                .ok_or_else(|| SttError::Api("ElevenLabs API key not configured".to_string()))?;
            elevenlabs::transcribe(audio, api_key).await
        }
        SttProvider::Google => {
            let credentials = config.google_credentials_json.as_ref()
                .ok_or_else(|| SttError::Api("Google credentials not configured".to_string()))?;
            google::transcribe(audio, credentials).await
        }
    }
}