use super::SttError;
use crate::audio::ConvertedAudio;
use log::{debug, info};
use serde::Deserialize;

#[derive(Deserialize)]
struct DgAlternative {
    transcript: String,
}

#[derive(Deserialize)]
struct DgChannel {
    alternatives: Vec<DgAlternative>,
}

#[derive(Deserialize)]
struct DgResults {
    channels: Vec<DgChannel>,
}

#[derive(Deserialize)]
struct DgResponse {
    results: DgResults,
}

#[derive(Deserialize)]
struct DgErrorResponse {
    err_msg: Option<String>,
    reason: Option<String>,
}

#[derive(Deserialize)]
struct DgProject {
    project_id: String,
}

#[derive(Deserialize)]
struct DgProjectsResp {
    projects: Vec<DgProject>,
}

#[derive(Deserialize, Clone)]
pub struct DgBalance {
    pub amount: f64,
    pub units: String,
}

#[derive(Deserialize)]
struct DgBalancesResp {
    balances: Vec<DgBalance>,
}

pub async fn transcribe(audio: &ConvertedAudio, api_key: &str) -> Result<String, SttError> {
    info!(
        "Starting transcription provider=deepgram model=nova-3 bytes={} format={}",
        audio.data.len(),
        audio.format
    );

    if audio.format != "pcm" {
        return Err(SttError::Api(
            "Deepgram module requires PCM format audio".to_string(),
        ));
    }

    let client = reqwest::Client::new();

    debug!("Sending request to Deepgram /v1/listen (nova-3)");

    let response = client
        .post("https://api.deepgram.com/v1/listen")
        .query(&[
            ("model", "nova-3"),
            ("smart_format", "true"),
            ("detect_language", "true"),
            ("encoding", "linear16"),
            ("sample_rate", "16000"),
            ("channels", "1"),
        ])
        .header("Authorization", format!("Token {}", api_key))
        .header("Content-Type", "audio/l16")
        .body(audio.data.clone())
        .send()
        .await?;

    let status = response.status();
    debug!("Deepgram API response status: {}", status);

    if status.is_success() {
        let body = response.text().await?;

        let dg: DgResponse = serde_json::from_str(&body)
            .map_err(|e| SttError::InvalidResponse(format!("Failed to parse Deepgram response: {}", e)))?;

        let transcript = dg
            .results
            .channels
            .into_iter()
            .next()
            .and_then(|ch| ch.alternatives.into_iter().next())
            .map(|alt| alt.transcript)
            .unwrap_or_default();

        info!(
            "Transcription complete provider=deepgram model=nova-3 chars={}",
            transcript.len()
        );
        Ok(transcript.trim().to_string())
    } else {
        let error_body = response.text().await?;

        let error_message = serde_json::from_str::<DgErrorResponse>(&error_body)
            .ok()
            .and_then(|e| e.err_msg.or(e.reason))
            .unwrap_or_else(|| error_body.clone());

        match status.as_u16() {
            401 => Err(SttError::Authentication),
            429 => Err(SttError::RateLimit),
            503 => Err(SttError::ServiceUnavailable),
            _ => Err(SttError::Api(error_message)),
        }
    }
}

pub async fn get_balance(api_key: &str) -> Result<DgBalance, SttError> {
    info!("Getting Deepgram balance");

    let client = reqwest::Client::new();
    let auth = format!("Token {}", api_key);

    let projects_resp = client
        .get("https://api.deepgram.com/v1/projects")
        .header("Authorization", &auth)
        .send()
        .await?;

    let status = projects_resp.status();
    if !status.is_success() {
        let body = projects_resp.text().await.unwrap_or_default();
        return Err(match status.as_u16() {
            401 => SttError::Authentication,
            429 => SttError::RateLimit,
            503 => SttError::ServiceUnavailable,
            _ => SttError::Api(format!("Deepgram projects: HTTP {}: {}", status, body)),
        });
    }

    let projects_body = projects_resp.text().await?;
    let projects: DgProjectsResp = serde_json::from_str(&projects_body).map_err(|e| {
        SttError::InvalidResponse(format!("Failed to parse Deepgram projects: {}", e))
    })?;

    let project_id = projects
        .projects
        .into_iter()
        .next()
        .map(|p| p.project_id)
        .ok_or_else(|| SttError::Api("No Deepgram projects found for this API key".to_string()))?;

    let balances_resp = client
        .get(format!(
            "https://api.deepgram.com/v1/projects/{}/balances",
            project_id
        ))
        .header("Authorization", &auth)
        .send()
        .await?;

    let status = balances_resp.status();
    if !status.is_success() {
        let body = balances_resp.text().await.unwrap_or_default();
        return Err(match status.as_u16() {
            401 => SttError::Authentication,
            429 => SttError::RateLimit,
            503 => SttError::ServiceUnavailable,
            _ => SttError::Api(format!("Deepgram balances: HTTP {}: {}", status, body)),
        });
    }

    let balances_body = balances_resp.text().await?;
    let balances: DgBalancesResp = serde_json::from_str(&balances_body).map_err(|e| {
        SttError::InvalidResponse(format!("Failed to parse Deepgram balances: {}", e))
    })?;

    balances
        .balances
        .into_iter()
        .next()
        .ok_or_else(|| SttError::Api("No balances returned for Deepgram project".to_string()))
}
