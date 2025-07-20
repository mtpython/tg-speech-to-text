pub mod convert;

pub use convert::*;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Unsupported audio format: {0}")]
    UnsupportedFormat(String),
    #[error("Audio conversion failed: {0}")]
    ConversionFailed(String),
    #[error("FFmpeg not found or not executable")]
    FfmpegNotFound,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Temp file error: {0}")]
    TempFile(String),
}