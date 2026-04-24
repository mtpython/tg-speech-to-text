pub mod elevenlabs;
pub mod whisper;
pub mod google;
pub mod deepgram;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SttProvider {
    Whisper,
    ElevenLabs,
    Google,
    Deepgram,
}

impl SttProvider {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "whisper" => Some(Self::Whisper),
            "elevenlabs" => Some(Self::ElevenLabs),
            "google" => Some(Self::Google),
            "deepgram" => Some(Self::Deepgram),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Whisper => "whisper",
            Self::ElevenLabs => "elevenlabs",
            Self::Google => "google",
            Self::Deepgram => "deepgram",
        }
    }

    pub fn model(&self) -> &'static str {
        match self {
            Self::Whisper => "whisper-1",
            Self::ElevenLabs => "scribe_v1_experimental",
            Self::Google => "default",
            Self::Deepgram => "nova-3",
        }
    }
}

pub async fn transcribe(
    audio: &ConvertedAudio,
    provider: SttProvider,
    config: &BotConfig,
) -> Result<String, SttError> {
    match provider {
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
        SttProvider::Deepgram => {
            let api_key = config.deepgram_api_key.as_ref()
                .ok_or_else(|| SttError::Api("Deepgram API key not configured".to_string()))?;
            deepgram::transcribe(audio, api_key).await
        }
    }
}
