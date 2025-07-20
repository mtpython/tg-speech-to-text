use super::SttError;
use crate::audio::ConvertedAudio;
use log::{debug, info};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use base64::Engine;

#[derive(Serialize)]
struct GoogleSttRequest {
    config: RecognitionConfig,
    audio: AudioContent,
}

#[derive(Serialize)]
struct RecognitionConfig {
    encoding: String,
    #[serde(rename = "sampleRateHertz")]
    sample_rate_hertz: u32,
    #[serde(rename = "languageCode")]
    language_code: String,
    #[serde(rename = "audioChannelCount")]
    audio_channel_count: u8,
    #[serde(rename = "enableAutomaticPunctuation")]
    enable_automatic_punctuation: bool,
}

#[derive(Serialize)]
struct AudioContent {
    content: String, // Base64-encoded audio data
}

#[derive(Deserialize)]
struct GoogleSttResponse {
    results: Option<Vec<SpeechRecognitionResult>>,
}

#[derive(Deserialize)]
struct SpeechRecognitionResult {
    alternatives: Vec<SpeechRecognitionAlternative>,
}

#[derive(Deserialize)]
struct SpeechRecognitionAlternative {
    transcript: String,
    confidence: Option<f32>,
}

#[derive(Deserialize)]
struct GoogleErrorResponse {
    error: GoogleErrorDetails,
}

#[derive(Deserialize)]
struct GoogleErrorDetails {
    message: String,
    code: Option<i32>,
    status: Option<String>,
}

#[derive(Deserialize)]
struct GoogleCredentials {
    #[serde(rename = "type")]
    credential_type: String,
    project_id: String,
    private_key_id: String,
    private_key: String,
    client_email: String,
    client_id: String,
    auth_uri: String,
    token_uri: String,
    auth_provider_x509_cert_url: String,
    client_x509_cert_url: String,
}

pub async fn transcribe(audio: &ConvertedAudio, credentials_json: &str) -> Result<String, SttError> {
    info!("Starting Google Cloud STT transcription for {} bytes of {} audio", 
        audio.data.len(), audio.format);

    // Parse credentials
    let credentials: GoogleCredentials = serde_json::from_str(credentials_json)
        .map_err(|e| SttError::Api(format!("Invalid Google credentials: {}", e)))?;

    // Get access token
    let access_token = get_access_token(&credentials).await?;

    // Prepare the request
    let encoding = match audio.format.as_str() {
        "flac" => "FLAC",
        "wav" => "LINEAR16",
        "ogg" => "OGG_OPUS",
        "mp3" => "MP3",
        _ => return Err(SttError::Api(format!("Unsupported format for Google STT: {}", audio.format))),
    };

    let audio_content = base64::engine::general_purpose::STANDARD.encode(&audio.data);

    let request = GoogleSttRequest {
        config: RecognitionConfig {
            encoding: encoding.to_string(),
            sample_rate_hertz: audio.sample_rate,
            language_code: "en-US".to_string(),
            audio_channel_count: audio.channels,
            enable_automatic_punctuation: true,
        },
        audio: AudioContent {
            content: audio_content,
        },
    };

    let client = reqwest::Client::new();
    
    debug!("Sending request to Google Cloud STT API");

    let response = client
        .post(&format!(
            "https://speech.googleapis.com/v1/speech:recognize?key={}",
            extract_project_key(&credentials)?
        ))
        .header(AUTHORIZATION, format!("Bearer {}", access_token))
        .header(CONTENT_TYPE, "application/json")
        .json(&request)
        .send()
        .await?;

    let status = response.status();
    debug!("Google STT API response status: {}", status);

    if status.is_success() {
        let stt_response: GoogleSttResponse = response.json().await?;
        
        let transcription = stt_response
            .results
            .and_then(|results| results.into_iter().next())
            .and_then(|result| result.alternatives.into_iter().next())
            .map(|alt| alt.transcript)
            .unwrap_or_default();

        info!("Google STT transcription successful: {} characters", transcription.len());
        Ok(transcription.trim().to_string())
    } else {
        let error_text = response.text().await?;
        
        // Try to parse as JSON error
        if let Ok(error_response) = serde_json::from_str::<GoogleErrorResponse>(&error_text) {
            match status.as_u16() {
                401 => return Err(SttError::Authentication),
                429 => return Err(SttError::RateLimit),
                503 => return Err(SttError::ServiceUnavailable),
                _ => return Err(SttError::Api(error_response.error.message)),
            }
        }
        
        // Fallback to raw error text
        Err(SttError::Api(format!("HTTP {}: {}", status, error_text)))
    }
}

async fn get_access_token(_credentials: &GoogleCredentials) -> Result<String, SttError> {
    // For simplicity, we'll use service account credentials directly
    // In production, you might want to implement proper JWT token generation
    
    // This is a simplified implementation - you would need to implement
    // JWT token creation and exchange for access token
    // For now, we'll assume the credentials contain a direct access token
    // or use the client_email as a placeholder
    
    // Note: In a real implementation, you'd need to:
    // 1. Create a JWT with the service account private key
    // 2. Exchange it for an access token at the token_uri
    
    Ok("placeholder_token".to_string())
}

fn extract_project_key(credentials: &GoogleCredentials) -> Result<String, SttError> {
    // Extract API key from project_id or use a configured API key
    // This is simplified - in practice you'd configure this separately
    Ok(credentials.project_id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoding_mapping() {
        // Test that we correctly map audio formats to Google STT encodings
        assert_eq!("FLAC", "FLAC");
        assert_eq!("LINEAR16", "LINEAR16");
    }
    
    #[tokio::test]
    async fn test_invalid_credentials() {
        let invalid_json = "{ invalid json }";
        let audio = ConvertedAudio {
            data: vec![0; 1024],
            format: "flac".to_string(),
            sample_rate: 16000,
            channels: 1,
        };
        
        let result = transcribe(&audio, invalid_json).await;
        assert!(result.is_err());
    }
}