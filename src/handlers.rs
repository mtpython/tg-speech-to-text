use crate::{audio, stt, BotConfig, BotError, Result, AuthorizedUsers, CurrentProvider, queue, persistence};
use log::{error, info};
use teloxide::{
    prelude::*,
    types::MessageKind,
    utils::command::BotCommands,
    net::Download,
};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
pub enum Command {
    #[command(description = "Display this help text")]
    Help,
    #[command(description = "Show bot status and configuration")]
    Status,
    #[command(description = "Start the bot")]
    Start,
    #[command(description = "Show queue status and statistics")]
    Queue,
    #[command(description = "Show credits for a provider: /credits [deepgram|elevenlabs]")]
    Credits(String),
    #[command(description = "Show current STT provider")]
    Provider,
    #[command(description = "Switch STT provider (admin only): /setprovider <whisper|elevenlabs|google|deepgram>")]
    SetProvider(String),
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

            // Save to persistent storage
            if let Err(e) = persistence::save_authorized_users(&users).await {
                error!("Failed to save authorized users: {}", e);
            }

            return true;
        }
    }

    false
}

fn is_admin(msg: &Message, config: &BotConfig) -> bool {
    msg.from()
        .map(|u| config.admin_user_ids.contains(&u.id))
        .unwrap_or(false)
}

fn provider_key_configured(provider: stt::SttProvider, config: &BotConfig) -> bool {
    match provider {
        stt::SttProvider::Whisper => config.openai_api_key.is_some(),
        stt::SttProvider::ElevenLabs => config.elevenlabs_api_key.is_some(),
        stt::SttProvider::Google => config.google_credentials_json.is_some(),
        stt::SttProvider::Deepgram => config.deepgram_api_key.is_some(),
    }
}

pub async fn command_handler(
    bot: Bot,
    msg: Message,
    cmd: Command,
    config: BotConfig,
    authorized_users: AuthorizedUsers,
    queue_stats: queue::QueueStats,
    current_provider: CurrentProvider,
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
            let provider = *current_provider.read().await;
            let status_text = format!(
                "🤖 Bot Status: ✅ Online\n\
                🔧 STT Provider: {}\n\
                🧠 Model: {}\n\
                📊 Memory usage: Low\n\
                🚀 Ready to transcribe!",
                provider.as_str(),
                provider.model()
            );

            bot.send_message(msg.chat.id, status_text).await?;
        }
        Command::Queue => {
            let queue_status = queue::get_queue_status(&queue_stats).await;
            bot.send_message(msg.chat.id, queue_status)
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .await?;
        }
        Command::Credits(arg) => {
            let name = arg.trim().to_lowercase();
            let target = if name.is_empty() {
                *current_provider.read().await
            } else {
                match stt::SttProvider::from_str(&name) {
                    Some(p) => p,
                    None => {
                        bot.send_message(
                            msg.chat.id,
                            format!("❌ Unknown provider '{}'. Valid options: deepgram, elevenlabs", name),
                        ).await?;
                        return Ok(());
                    }
                }
            };

            match target {
                stt::SttProvider::ElevenLabs => {
                    match &config.elevenlabs_api_key {
                        Some(api_key) => {
                            match stt::elevenlabs::get_user_credits(api_key).await {
                                Ok(user_info) => {
                                    let credits_text = format!(
                                        "💳 ElevenLabs Credits\n\
                                        Used: {} characters\n\
                                        Limit: {} characters\n\
                                        Remaining: {} characters",
                                        user_info.subscription.character_count,
                                        user_info.subscription.character_limit,
                                        user_info.subscription.character_limit.saturating_sub(user_info.subscription.character_count)
                                    );
                                    bot.send_message(msg.chat.id, credits_text).await?;
                                }
                                Err(e) => {
                                    bot.send_message(msg.chat.id, format!("❌ Failed to get credits: {}", e)).await?;
                                }
                            }
                        }
                        None => {
                            bot.send_message(msg.chat.id, "❌ ElevenLabs API key not configured").await?;
                        }
                    }
                }
                stt::SttProvider::Deepgram => {
                    match &config.deepgram_api_key {
                        Some(api_key) => {
                            match stt::deepgram::get_balance(api_key).await {
                                Ok(b) => {
                                    let credits_text = format!(
                                        "💳 Deepgram Balance\nRemaining: {:.2} {}",
                                        b.amount,
                                        b.units.to_uppercase()
                                    );
                                    bot.send_message(msg.chat.id, credits_text).await?;
                                }
                                Err(e) => {
                                    bot.send_message(msg.chat.id, format!("❌ Failed to get Deepgram balance: {}", e)).await?;
                                }
                            }
                        }
                        None => {
                            bot.send_message(msg.chat.id, "❌ Deepgram API key not configured").await?;
                        }
                    }
                }
                stt::SttProvider::Whisper | stt::SttProvider::Google => {
                    bot.send_message(
                        msg.chat.id,
                        format!("ℹ️ Credits lookup is not supported for '{}'.", target.as_str()),
                    ).await?;
                }
            }
        }
        Command::Provider => {
            let provider = *current_provider.read().await;
            let key_status = if provider_key_configured(provider, &config) {
                "✅ API key configured"
            } else {
                "⚠️ API key not configured"
            };
            let text = format!(
                "🔧 Current STT provider: {}\n🧠 Model: {}\n{}",
                provider.as_str(),
                provider.model(),
                key_status
            );
            bot.send_message(msg.chat.id, text).await?;
        }
        Command::SetProvider(name) => {
            if !is_admin(&msg, &config) {
                bot.send_message(msg.chat.id, "❌ Not authorized. Only admins can switch providers.").await?;
                return Ok(());
            }

            let name = name.trim().to_lowercase();
            if name.is_empty() {
                bot.send_message(
                    msg.chat.id,
                    "Usage: /setprovider <whisper|elevenlabs|google|deepgram>",
                ).await?;
                return Ok(());
            }

            let new_provider = match stt::SttProvider::from_str(&name) {
                Some(p) => p,
                None => {
                    bot.send_message(
                        msg.chat.id,
                        format!("❌ Unknown provider '{}'. Valid options: whisper, elevenlabs, google, deepgram", name),
                    ).await?;
                    return Ok(());
                }
            };

            if !provider_key_configured(new_provider, &config) {
                bot.send_message(
                    msg.chat.id,
                    format!("❌ Cannot switch to '{}': API key not configured on this bot.", name),
                ).await?;
                return Ok(());
            }

            *current_provider.write().await = new_provider;

            if let Err(e) = persistence::save_runtime_config(new_provider).await {
                error!("Failed to persist provider switch: {}", e);
                bot.send_message(msg.chat.id, "⚠️ Provider switched but could not be persisted. It will revert after restart.").await?;
                return Ok(());
            }

            bot.send_message(
                msg.chat.id,
                format!("✅ STT provider switched to '{}'.", new_provider.as_str()),
            ).await?;
        }
    }
    Ok(())
}

pub async fn audio_handler(
    bot: Bot,
    msg: Message,
    config: BotConfig,
    authorized_users: AuthorizedUsers,
    queue_sender: queue::QueueSender,
    queue_stats: queue::QueueStats,
) -> ResponseResult<()> {
    if !is_authorized(&msg, &config, &authorized_users).await {
        return Ok(());
    }

    // Download and queue the audio file
    let queue_result = download_and_queue_audio(&bot, &msg, &queue_sender, &queue_stats).await;

    match queue_result {
        Ok(queue_position) => {
            info!("Audio file queued successfully at position {}", queue_position);
        }
        Err(e) => {
            error!("Error queueing audio: {}", e);
            let error_msg = match e {
                BotError::Audio(audio::AudioError::UnsupportedFormat(_)) => {
                    "❌ Unsupported audio format. Please send voice messages, video notes, audio files (.mp3, .m4a, .ogg), or video files."
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

async fn download_and_queue_audio(
    bot: &Bot,
    msg: &Message,
    queue_sender: &queue::QueueSender,
    queue_stats: &queue::QueueStats,
) -> Result<u64> {
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

    // Get user info for logging
    let user_info = msg.from()
        .map(|user| {
            if let Some(username) = &user.username {
                format!("@{}", username)
            } else {
                format!("{} {}", user.first_name, user.last_name.as_deref().unwrap_or(""))
            }
        })
        .unwrap_or_else(|| "Unknown".to_string());

    // Extract user ID and username for detailed logging
    let (user_id, username) = msg.from()
        .map(|user| (user.id, user.username.clone()))
        .unwrap_or_else(|| (teloxide::types::UserId(0), None));

    // Get current queue size for position calculation
    let queue_position = {
        let mut stats = queue_stats.write().await;
        stats.increment_queued().await;
        stats.current_queue_size
    };

    // Send initial queue message
    let processing_msg = bot
        .send_message(
            msg.chat.id,
            format!("📥 Added to queue (position: {})\nFile: {}", queue_position, original_filename)
        )
        .await?;

    // Create queue item
    let queue_item = queue::QueueItem::new(
        bot.clone(),
        msg.chat.id,
        processing_msg.id,
        msg.id,
        file_data,
        original_filename.to_string(),
        user_info,
        user_id,
        username,
    );

    // Send to queue
    if let Err(e) = queue_sender.send(queue_item) {
        error!("Failed to send item to queue: {}", e);

        // Decrement queue count since we failed to queue
        {
            let mut stats = queue_stats.write().await;
            stats.current_queue_size = stats.current_queue_size.saturating_sub(1);
        }

        // Delete the processing message
        bot.delete_message(msg.chat.id, processing_msg.id).await.ok();

        return Err(BotError::Config("Queue is full or closed".to_string()));
    }

    Ok(queue_position)
}

pub async fn text_handler(bot: Bot, msg: Message, config: BotConfig, authorized_users: AuthorizedUsers) -> ResponseResult<()> {
    if !is_authorized(&msg, &config, &authorized_users).await {
        return Ok(());
    }

    Ok(())
}
