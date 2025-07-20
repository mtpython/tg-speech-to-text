use super::SttError;
use crate::audio::ConvertedAudio;
use log::{debug, info};
use reqwest::multipart;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct WhisperRequest {
    model: String,
    response_format: String,
    temperature: f32,
}

#[derive(Deserialize)]
struct WhisperResponse {
    text: String,
}

#[derive(Deserialize)]
struct WhisperErrorResponse {
    error: WhisperErrorDetails,
}

#[derive(Deserialize)]
struct WhisperErrorDetails {
    message: String,
    #[serde(rename = "type")]
    error_type: Option<String>,
    code: Option<String>,
}

pub async fn transcribe(audio: &ConvertedAudio, api_key: &str) -> Result<String, SttError> {
    info!("Starting Whisper transcription for {} bytes of {} audio", 
        audio.data.len(), audio.format);

    let client = reqwest::Client::new();
    
    // Prepare the file part - Whisper expects the file to have proper extension
    let filename = match audio.format.as_str() {
        "wav" => "audio.wav",
        "mp3" => "audio.mp3",
        "flac" => "audio.flac",
        "ogg" => "audio.ogg",
        _ => "audio.wav", // Default to wav
    };

    // Create multipart form
    let file_part = multipart::Part::bytes(audio.data.clone())
        .file_name(filename.to_string())
        .mime_str(get_mime_type(&audio.format))
        .map_err(|e| SttError::InvalidResponse(format!("Invalid mime type: {}", e)))?;

    let form = multipart::Form::new()
        .part("file", file_part)
        .text("model", "whisper-1")
        .text("response_format", "text")
        .text("temperature", "0.0");

    debug!("Sending request to OpenAI Whisper API");

    let response = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await?;

    let status = response.status();
    debug!("Whisper API response status: {}", status);

    if status.is_success() {
        let transcription = response.text().await?;
        info!("Whisper transcription successful: {} characters", transcription.len());
        Ok(transcription.trim().to_string())
    } else {
        let error_text = response.text().await?;
        
        // Try to parse as JSON error
        if let Ok(error_response) = serde_json::from_str::<WhisperErrorResponse>(&error_text) {
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

fn get_mime_type(format: &str) -> &'static str {
    match format {
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        "flac" => "audio/flac",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        _ => "audio/wav",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mime_type_mapping() {
        assert_eq!(get_mime_type("wav"), "audio/wav");
        assert_eq!(get_mime_type("mp3"), "audio/mpeg");
        assert_eq!(get_mime_type("flac"), "audio/flac");
        assert_eq!(get_mime_type("unknown"), "audio/wav");
    }
}