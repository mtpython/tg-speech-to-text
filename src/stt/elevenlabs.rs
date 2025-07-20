use super::SttError;
use crate::audio::ConvertedAudio;
use log::{debug, info};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::Deserialize;

#[derive(Deserialize)]
struct ElevenLabsResponse {
    text: String,
    #[serde(default)]
    success: bool,
}

#[derive(Deserialize)]
struct ElevenLabsErrorResponse {
    detail: Option<String>,
    message: Option<String>,
}

pub async fn transcribe(audio: &ConvertedAudio, api_key: &str) -> Result<String, SttError> {
    info!("Starting ElevenLabs transcription for {} bytes of {} audio", 
        audio.data.len(), audio.format);

    // ElevenLabs expects PCM 16kHz mono data
    if audio.format != "pcm" {
        return Err(SttError::Api(
            "ElevenLabs requires PCM format audio".to_string()
        ));
    }

    let client = reqwest::Client::new();
    
    // Prepare headers
    let mut headers = HeaderMap::new();
    headers.insert("xi-api-key", HeaderValue::from_str(api_key)
        .map_err(|e| SttError::Api(format!("Invalid API key format: {}", e)))?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("audio/wav"));

    debug!("Sending request to ElevenLabs STT API");

    let response = client
        .post("https://api.elevenlabs.io/v1/speech-to-text")
        .headers(headers)
        .body(audio.data.clone())
        .send()
        .await?;

    let status = response.status();
    debug!("ElevenLabs API response status: {}", status);

    if status.is_success() {
        let response_text = response.text().await?;
        
        // Try to parse as JSON first
        if let Ok(stt_response) = serde_json::from_str::<ElevenLabsResponse>(&response_text) {
            info!("ElevenLabs transcription successful: {} characters", stt_response.text.len());
            return Ok(stt_response.text.trim().to_string());
        }
        
        // If not JSON, treat as plain text
        info!("ElevenLabs transcription successful (plain text): {} characters", response_text.len());
        Ok(response_text.trim().to_string())
    } else {
        let error_text = response.text().await?;
        
        // Try to parse as JSON error
        if let Ok(error_response) = serde_json::from_str::<ElevenLabsErrorResponse>(&error_text) {
            let error_message = error_response.detail
                .or(error_response.message)
                .unwrap_or_else(|| "Unknown error".to_string());
            
            match status.as_u16() {
                401 => return Err(SttError::Authentication),
                429 => return Err(SttError::RateLimit),
                503 => return Err(SttError::ServiceUnavailable),
                _ => return Err(SttError::Api(error_message)),
            }
        }
        
        // Fallback to raw error text
        Err(SttError::Api(format!("HTTP {}: {}", status, error_text)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transcribe_invalid_format() {
        let audio = ConvertedAudio {
            data: vec![0; 1024],
            format: "mp3".to_string(),
            sample_rate: 16000,
            channels: 1,
        };
        
        let result = transcribe(&audio, "test_key").await;
        assert!(result.is_err());
        
        if let Err(SttError::Api(msg)) = result {
            assert!(msg.contains("PCM format"));
        }
    }
}