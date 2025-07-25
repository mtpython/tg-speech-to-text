use crate::{audio, stt, BotConfig, BotError, Result, AuthorizedUsers};
use log::{error, info};
use teloxide::{
    prelude::*,
    types::MessageKind,
    utils::command::BotCommands,
    net::Download,
};
use std::time::Instant;

/// Escapes special characters for Telegram MarkdownV2 format
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

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
pub enum Command {
    #[command(description = "Display this help text")]
    Help,
    #[command(description = "Show bot status and configuration")]
    Status,
    #[command(description = "Start the bot")]
    Start,
}

async fn is_authorized(msg: &Message, config: &BotConfig, authorized_users: &AuthorizedUsers) -> bool {
    let user_id = match msg.from() {
        Some(user) => user.id,
        None => return false,
    };
    
    // If no password is configured, allow all users
    let Some(password) = &config.bot_password else {
        return true;
    };
    
    // Check if user is already authorized
    {
        let users = authorized_users.read().await;
        if users.contains(&user_id) {
            return true;
        }
    }
    
    // Check if current message is the password
    if let Some(text) = msg.text() {
        if text == password {
            // Authorize the user
            let mut users = authorized_users.write().await;
            users.insert(user_id);
            return true;
        }
    }
    
    false
}

pub async fn command_handler(
    bot: Bot,
    msg: Message,
    cmd: Command,
    config: BotConfig,
    authorized_users: AuthorizedUsers,
) -> ResponseResult<()> {
    if !is_authorized(&msg, &config, &authorized_users).await {
        return Ok(());
    }
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Start => {
            let welcome_text = "🎤 Welcome to the Speech-to-Text Bot!\n\n\
                📝 Send me:\n\
                • Voice messages\n\
                • Video notes (round video messages)\n\
                • Audio files (.mp3, .m4a, .ogg, etc.)\n\
                • Video files (I'll extract the audio)\n\n\
                I'll transcribe the speech and send you the text!";
            
            bot.send_message(msg.chat.id, welcome_text).await?;
        }
        Command::Status => {
            let status_text = format!(
                "🤖 Bot Status: ✅ Online\n\
                🔧 STT Provider: {:?}\n\
                📊 Memory usage: Low\n\
                🚀 Ready to transcribe!",
                config.stt_provider
            );
            
            bot.send_message(msg.chat.id, status_text).await?;
        }
    }
    Ok(())
}

pub async fn audio_handler(bot: Bot, msg: Message, config: BotConfig, authorized_users: AuthorizedUsers) -> ResponseResult<()> {
    if !is_authorized(&msg, &config, &authorized_users).await {
        return Ok(());
    }
    
    let start_time = Instant::now();
    
    // Send initial processing message
    let processing_msg = bot
        .send_message(msg.chat.id, "🎵 Processing audio... This may take a moment.")
        .await?;

    let result = process_audio_message(&bot, &msg, &config).await;
    
    // Delete the processing message
    bot.delete_message(msg.chat.id, processing_msg.id).await.ok();
    
    match result {
        Ok(transcription) => {
            let duration = start_time.elapsed();
            info!("Transcription completed in {:?}", duration);
            
            let response = if transcription.trim().is_empty() {
                "🔇 No speech detected in the audio\\. The audio might be too quiet or contain no spoken words\\.".to_string()
            } else {
                format!("📝 *Transcription:*\n\n{}", escape_markdown_v2(&transcription))
            };
            
            bot.send_message(msg.chat.id, response)
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .reply_to_message_id(msg.id)
                .await?;
        }
        Err(e) => {
            error!("Error processing audio: {}", e);
            let error_msg = match e {
                BotError::Audio(audio::AudioError::UnsupportedFormat(_)) => {
                    "❌ Unsupported audio format. Please send voice messages, video notes, audio files (.mp3, .m4a, .ogg), or video files."
                }
                BotError::Audio(audio::AudioError::ConversionFailed(_)) => {
                    "❌ Failed to process audio. The file might be corrupted or in an unsupported format."
                }
                BotError::Stt(_) => {
                    "❌ Speech-to-text service is temporarily unavailable. Please try again later."
                }
                _ => "❌ An error occurred while processing your audio. Please try again."
            };
            
            bot.send_message(msg.chat.id, error_msg)
                .reply_to_message_id(msg.id)
                .await?;
        }
    }
    
    Ok(())
}

async fn process_audio_message(bot: &Bot, msg: &Message, config: &BotConfig) -> Result<String> {
    let (file_ref, original_filename) = match &msg.kind {
        MessageKind::Common(common) => {
            match &common.media_kind {
                teloxide::types::MediaKind::Voice(voice_msg) => {
                    info!("Processing voice message: duration {}s", voice_msg.voice.duration);
                    (&voice_msg.voice.file, "voice.ogg")
                }
                teloxide::types::MediaKind::Audio(audio_msg) => {
                    info!("Processing audio file: {} ({}s)", 
                        audio_msg.audio.file_name.as_deref().unwrap_or("unknown"), 
                        audio_msg.audio.duration
                    );
                    let filename = audio_msg.audio.file_name.as_deref().unwrap_or("audio.mp3");
                    (&audio_msg.audio.file, filename)
                }
                teloxide::types::MediaKind::Video(video_msg) => {
                    info!("Processing video file: duration {}s", video_msg.video.duration);
                    (&video_msg.video.file, "video.mp4")
                }
                teloxide::types::MediaKind::VideoNote(video_note_msg) => {
                    info!("Processing video note: duration {}s", video_note_msg.video_note.duration);
                    (&video_note_msg.video_note.file, "video_note.mp4")
                }
                teloxide::types::MediaKind::Document(doc_msg) => {
                    info!("Processing document: {}", 
                        doc_msg.document.file_name.as_deref().unwrap_or("unknown"));
                    let filename = doc_msg.document.file_name.as_deref().unwrap_or("document.bin");
                    (&doc_msg.document.file, filename)
                }
                _ => {
                    return Err(BotError::Config("Unsupported media type".to_string()));
                }
            }
        }
        _ => {
            return Err(BotError::Config("Message is not a common type".to_string()));
        }
    };

    // Download the file
    info!("Downloading file: {}", file_ref.id);
    let file = bot.get_file(&file_ref.id).await?;
    
    let mut file_data = Vec::new();
    bot.download_file(&file.path, &mut file_data).await?;
    
    info!("Downloaded {} bytes", file_data.len());

    // Convert audio to the format required by the STT provider
    let converted_audio = audio::convert_for_stt(&file_data, original_filename, config.stt_provider).await?;
    
    // Transcribe using the configured STT provider
    let transcription = stt::transcribe(&converted_audio, config).await?;
    
    Ok(transcription)
}

pub async fn text_handler(bot: Bot, msg: Message, config: BotConfig, authorized_users: AuthorizedUsers) -> ResponseResult<()> {
    if !is_authorized(&msg, &config, &authorized_users).await {
        return Ok(());
    }
    
    Ok(())
}