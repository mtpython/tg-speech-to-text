mod handlers;
mod stt;
mod audio;

use dotenvy::dotenv;
use log::{error, info};
use std::env;
use teloxide::{prelude::*, Bot};
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

#[derive(Clone)]
pub struct BotConfig {
    pub telegram_token: String,
    pub stt_provider: stt::SttProvider,
    pub elevenlabs_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub google_credentials_json: Option<String>,
}

impl BotConfig {
    pub fn from_env() -> Result<Self> {
        let telegram_token = env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| BotError::Config("TELEGRAM_BOT_TOKEN not set".to_string()))?;

        let stt_provider_str = env::var("STT_PROVIDER").unwrap_or_else(|_| "whisper".to_string());
        let stt_provider = match stt_provider_str.as_str() {
            "whisper" => stt::SttProvider::Whisper,
            "elevenlabs" => stt::SttProvider::ElevenLabs,
            "google" => stt::SttProvider::Google,
            _ => return Err(BotError::Config("Invalid STT_PROVIDER".to_string())),
        };

        let elevenlabs_api_key = env::var("ELEVENLABS_API_KEY").ok();
        let openai_api_key = env::var("OPENAI_API_KEY").ok();
        let google_credentials_json = env::var("GOOGLE_CREDENTIALS_JSON").ok();

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
            _ => {}
        }

        Ok(BotConfig {
            telegram_token,
            stt_provider,
            elevenlabs_api_key,
            openai_api_key,
            google_credentials_json,
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
    info!("Using STT provider: {:?}", config.stt_provider);

    // Create bot instance
    let bot = Bot::new(&config.telegram_token);

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
                    msg.voice().is_some() || msg.audio().is_some() || msg.video().is_some()
                }))
                .endpoint(handlers::audio_handler),
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
            .run(([0, 0, 0, 0], 8080))
            .await;
    });

    info!("Health check server started on port 8080");

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![config])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    info!("Bot stopped");
    Ok(())
}
