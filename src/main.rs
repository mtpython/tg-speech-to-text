mod handlers;
mod stt;
mod audio;
mod queue;
mod persistence;
mod request_logger;

use dotenvy::dotenv;
use log::{error, info};
use std::env;
use std::sync::Arc;
use std::collections::HashSet;
use tokio::sync::{RwLock, mpsc};
use teloxide::{prelude::*, Bot, types::UserId};
use thiserror::Error;
use warp::Filter;

#[derive(Error, Debug)]
pub enum BotError {
    #[error("Telegram error: {0}")]
    Telegram(#[from] teloxide::RequestError),
    #[error("STT provider error: {0}")]
    Stt(#[from] stt::SttError),
    #[error("Audio processing error: {0}")]
    Audio(#[from] audio::AudioError),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Download error: {0}")]
    Download(#[from] teloxide::DownloadError),
    #[error("Configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, BotError>;

pub type AuthorizedUsers = Arc<RwLock<HashSet<UserId>>>;
pub type CurrentProvider = Arc<RwLock<stt::SttProvider>>;

#[derive(Clone)]
pub struct BotConfig {
    pub telegram_token: String,
    pub stt_provider: stt::SttProvider,
    pub elevenlabs_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub google_credentials_json: Option<String>,
    pub deepgram_api_key: Option<String>,
    pub bot_password: Option<String>,
    pub admin_user_ids: HashSet<UserId>,
}

impl BotConfig {
    pub fn from_env() -> Result<Self> {
        let telegram_token = env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| BotError::Config("TELEGRAM_BOT_TOKEN not set".to_string()))?;

        let stt_provider_str = env::var("STT_PROVIDER").unwrap_or_else(|_| "deepgram".to_string());
        let stt_provider = stt::SttProvider::from_str(&stt_provider_str)
            .ok_or_else(|| BotError::Config(format!("Invalid STT_PROVIDER: {}", stt_provider_str)))?;

        let elevenlabs_api_key = env::var("ELEVENLABS_API_KEY").ok();
        let openai_api_key = env::var("OPENAI_API_KEY").ok();
        let google_credentials_json = env::var("GOOGLE_CREDENTIALS_JSON").ok();
        let deepgram_api_key = env::var("DEEPGRAM_API_KEY").ok();
        let bot_password = env::var("BOT_PASSWORD").ok();

        let admin_user_ids: HashSet<UserId> = env::var("ADMIN_USER_IDS")
            .unwrap_or_default()
            .split(',')
            .filter_map(|s| s.trim().parse::<u64>().ok())
            .map(UserId)
            .collect();

        // Validate that required API keys are present for selected provider
        match stt_provider {
            stt::SttProvider::Whisper if openai_api_key.is_none() => {
                return Err(BotError::Config("OPENAI_API_KEY required for Whisper".to_string()));
            }
            stt::SttProvider::ElevenLabs if elevenlabs_api_key.is_none() => {
                return Err(BotError::Config("ELEVENLABS_API_KEY required for ElevenLabs".to_string()));
            }
            stt::SttProvider::Google if google_credentials_json.is_none() => {
                return Err(BotError::Config("GOOGLE_CREDENTIALS_JSON required for Google".to_string()));
            }
            stt::SttProvider::Deepgram if deepgram_api_key.is_none() => {
                return Err(BotError::Config("DEEPGRAM_API_KEY required for Deepgram".to_string()));
            }
            _ => {}
        }

        Ok(BotConfig {
            telegram_token,
            stt_provider,
            elevenlabs_api_key,
            openai_api_key,
            google_credentials_json,
            deepgram_api_key,
            bot_password,
            admin_user_ids,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Load environment variables
    dotenv().ok();

    info!("Starting Telegram STT Bot");

    // Load configuration
    let config = BotConfig::from_env()?;
    info!("Using STT provider (env): {:?}", config.stt_provider);

    // Create bot instance
    let bot = Bot::new(&config.telegram_token);

    // Load authorized users from persistent storage
    let initial_users = persistence::load_authorized_users().await?;
    let authorized_users: AuthorizedUsers = Arc::new(RwLock::new(initial_users));

    // Determine active provider: persisted runtime config overrides env
    let initial_provider = match persistence::load_runtime_config().await? {
        Some(persisted) => {
            info!("Runtime config overrides env provider: {:?}", persisted);
            persisted
        }
        None => config.stt_provider,
    };
    let current_provider: CurrentProvider = Arc::new(RwLock::new(initial_provider));

    // Create queue system
    let (queue_sender, queue_receiver) = mpsc::unbounded_channel();
    let queue_stats = Arc::new(RwLock::new(queue::QueueStatistics::default()));

    // Start queue processor in background
    let config_clone = config.clone();
    let stats_clone = queue_stats.clone();
    let provider_clone = current_provider.clone();
    tokio::spawn(async move {
        queue::start_queue_processor(queue_receiver, config_clone, stats_clone, provider_clone).await;
    });

    // Set up dispatcher
    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<handlers::Command>()
                .endpoint(handlers::command_handler),
        )
        .branch(
            Update::filter_message()
                .chain(dptree::filter(|msg: Message| {
                    msg.voice().is_some() || msg.audio().is_some() || msg.video().is_some() || msg.video_note().is_some()
                }))
                .endpoint(handlers::audio_handler),
        )
        .branch(
            Update::filter_message()
                .endpoint(handlers::text_handler),
        );

    info!("Bot started. Listening for messages...");

    // Start health check server
    let health_route = warp::path("health")
        .and(warp::get())
        .map(|| warp::reply::with_status("OK", warp::http::StatusCode::OK));

    let metrics_route = warp::path("metrics")
        .and(warp::get())
        .map(|| "# Telegram STT Bot Metrics\n# (Add your metrics here)\n");

    let routes = health_route.or(metrics_route);

    // Start health check server in background
    tokio::spawn(async move {
        warp::serve(routes)
            .run(([0, 0, 0, 0], 8091))
            .await;
    });

    info!("Health check server started on port 8091");

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![config, authorized_users, queue_sender, queue_stats, current_provider])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    info!("Bot stopped");
    Ok(())
}
