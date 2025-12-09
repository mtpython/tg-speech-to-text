use std::path::Path;
use log::{info, error};
use chrono::{DateTime, Utc};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use teloxide::types::UserId;
use crate::{BotError, Result};

const LOGS_DIR: &str = "data/logs";
const LOG_FILE: &str = "data/logs/transcription_requests.log";

pub async fn log_transcription_request(
    user_id: UserId,
    username: Option<&str>,
    audio_length: usize,
) -> Result<()> {
    // Create logs directory if it doesn't exist
    if let Some(parent) = Path::new(LOG_FILE).parent() {
        if !parent.exists() {
            tokio::fs::create_dir_all(parent).await.map_err(BotError::Io)?;
            info!("Created logs directory: {}", parent.display());
        }
    }

    // Format timestamp
    let now: DateTime<Utc> = Utc::now();
    let timestamp = now.format("%Y-%m-%d-%H-%M-%S").to_string();

    // Format log entry
    let log_entry = if let Some(username) = username {
        format!("{}, {}, {}, {}\n", timestamp, user_id.0, username, audio_length)
    } else {
        format!("{}, {}, , {}\n", timestamp, user_id.0, audio_length)
    };

    // Append to log file
    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE)
        .await
    {
        Ok(mut file) => {
            if let Err(e) = file.write_all(log_entry.as_bytes()).await {
                error!("Failed to write to transcription log: {}", e);
                return Err(BotError::Io(e));
            }

            if let Err(e) = file.flush().await {
                error!("Failed to flush transcription log: {}", e);
                return Err(BotError::Io(e));
            }

            info!("Logged transcription request for user {}: {} bytes", user_id.0, audio_length);
            Ok(())
        }
        Err(e) => {
            error!("Failed to open transcription log file: {}", e);
            Err(BotError::Io(e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_log_transcription_request() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().join("test_log.txt");

        // This is a basic test structure - actual testing would require
        // modifying the module to accept custom log paths
    }
}