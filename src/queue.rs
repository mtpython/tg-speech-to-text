use crate::{BotConfig, Result, BotError, request_logger, stt::SttProvider};
use log::{info, error, warn};
use std::sync::Arc;
use teloxide::{prelude::*, types::MessageId};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

#[derive(Clone)]
pub struct QueueItem {
    pub id: String,
    pub bot: Bot,
    pub chat_id: ChatId,
    pub message_id: MessageId,
    pub reply_to_message_id: MessageId,
    pub file_data: Vec<u8>,
    pub original_filename: String,
    pub user_info: String,
    pub user_id: teloxide::types::UserId,
    pub username: Option<String>,
}

impl QueueItem {
    pub fn new(
        bot: Bot,
        chat_id: ChatId,
        message_id: MessageId,
        reply_to_message_id: MessageId,
        file_data: Vec<u8>,
        original_filename: String,
        user_info: String,
        user_id: teloxide::types::UserId,
        username: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            bot,
            chat_id,
            message_id,
            reply_to_message_id,
            file_data,
            original_filename,
            user_info,
            user_id,
            username,
        }
    }
}

pub type QueueSender = mpsc::UnboundedSender<QueueItem>;
pub type QueueReceiver = mpsc::UnboundedReceiver<QueueItem>;
pub type QueueStats = Arc<RwLock<QueueStatistics>>;

#[derive(Default)]
pub struct QueueStatistics {
    pub total_queued: u64,
    pub total_processed: u64,
    pub total_failed: u64,
    pub current_queue_size: u64,
    pub processing_item_id: Option<String>,
}

impl QueueStatistics {
    pub async fn increment_queued(&mut self) {
        self.total_queued += 1;
        self.current_queue_size += 1;
    }

    pub async fn increment_processed(&mut self) {
        self.total_processed += 1;
        self.current_queue_size = self.current_queue_size.saturating_sub(1);
        self.processing_item_id = None;
    }

    pub async fn increment_failed(&mut self) {
        self.total_failed += 1;
        self.current_queue_size = self.current_queue_size.saturating_sub(1);
        self.processing_item_id = None;
    }

    pub async fn set_processing(&mut self, item_id: String) {
        self.processing_item_id = Some(item_id);
    }
}

pub async fn start_queue_processor(
    mut receiver: QueueReceiver,
    config: BotConfig,
    stats: QueueStats,
) {
    info!("Starting queue processor worker");

    while let Some(item) = receiver.recv().await {
        info!(
            "Processing queue item {} for user {} (file: {}, size: {} bytes)",
            item.id, item.user_info, item.original_filename, item.file_data.len()
        );

        // Update stats
        {
            let mut stats_guard = stats.write().await;
            stats_guard.set_processing(item.id.clone()).await;
        }

        // Update the processing message
        if let Err(e) = item.bot
            .edit_message_text(
                item.chat_id,
                item.message_id,
                format!("🎵 Processing audio... (Queue position: processing)\nFile: {}", item.original_filename)
            )
            .await
        {
            warn!("Failed to update processing message: {}", e);
        }

        // Process the audio
        let result = process_audio_item(&item, &config).await;

        // Delete the processing message
        item.bot.delete_message(item.chat_id, item.message_id).await.ok();

        // Send result
        match result {
            Ok(transcription) => {
                info!("Successfully processed queue item {}", item.id);

                let response = if transcription.trim().is_empty() {
                    "🔇 No speech detected in the audio\\. The audio might be too quiet or contain no spoken words\\.".to_string()
                } else {
                    format!("📝 *Transcription:*\n\n{}", escape_markdown_v2(&transcription))
                };

                if let Err(e) = item.bot
                    .send_message(item.chat_id, response)
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .reply_to_message_id(item.reply_to_message_id)
                    .await
                {
                    error!("Failed to send transcription for item {}: {}", item.id, e);
                }

                // Update stats
                {
                    let mut stats_guard = stats.write().await;
                    stats_guard.increment_processed().await;
                }
            }
            Err(e) => {
                error!("Failed to process queue item {}: {}", item.id, e);

                let error_msg = match e {
                    BotError::Audio(crate::audio::AudioError::UnsupportedFormat(_)) => {
                        "❌ Unsupported audio format. Please send voice messages, video notes, audio files (.mp3, .m4a, .ogg), or video files."
                    }
                    BotError::Audio(crate::audio::AudioError::ConversionFailed(_)) => {
                        "❌ Failed to process audio. The file might be corrupted or in an unsupported format."
                    }
                    BotError::Stt(_) => {
                        "❌ Speech-to-text service is temporarily unavailable. Please try again later."
                    }
                    _ => "❌ An error occurred while processing your audio. Please try again."
                };

                if let Err(e) = item.bot
                    .send_message(item.chat_id, error_msg)
                    .reply_to_message_id(item.reply_to_message_id)
                    .await
                {
                    error!("Failed to send error message for item {}: {}", item.id, e);
                }

                // Update stats
                {
                    let mut stats_guard = stats.write().await;
                    stats_guard.increment_failed().await;
                }
            }
        }
    }

    warn!("Queue processor stopped - receiver closed");
}

async fn process_audio_item(item: &QueueItem, config: &BotConfig) -> Result<String> {
    use crate::{audio, stt};

    // Log transcription request for ElevenLabs
    if matches!(config.stt_provider, SttProvider::ElevenLabs) {
        if let Err(e) = request_logger::log_transcription_request(
            item.user_id,
            item.username.as_deref(),
            item.file_data.len(),
        ).await {
            error!("Failed to log transcription request: {}", e);
        }
    }

    // Convert audio to the format required by the STT provider
    let converted_audio = audio::convert_for_stt(&item.file_data, &item.original_filename, config.stt_provider).await?;

    // Transcribe using the configured STT provider
    let transcription = stt::transcribe(&converted_audio, config).await?;

    Ok(transcription)
}

fn escape_markdown_v2(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '_' | '*' | '[' | ']' | '(' | ')' | '~' | '`' | '>' | '#' | '+' | '-' | '=' | '|' | '{' | '}' | '.' | '!' => {
                format!("\\{}", c)
            }
            _ => c.to_string(),
        })
        .collect()
}

pub async fn get_queue_status(stats: &QueueStats) -> String {
    let stats_guard = stats.read().await;

    let processing_info = if let Some(ref item_id) = stats_guard.processing_item_id {
        format!("Currently processing: {}", &item_id[..8])
    } else {
        "Idle".to_string()
    };

    format!(
        "🔄 *Queue Status:*\n\
        📊 Current queue size: {}\n\
        ⚙️ Status: {}\n\
        ✅ Total processed: {}\n\
        ❌ Total failed: {}\n\
        📥 Total queued: {}",
        stats_guard.current_queue_size,
        processing_info,
        stats_guard.total_processed,
        stats_guard.total_failed,
        stats_guard.total_queued
    )
}