use super::AudioError;
use crate::stt::SttProvider;
use log::{debug, info};
use std::process::Command;
use tempfile::NamedTempFile;
use std::io::Write;
use std::fs;

pub struct ConvertedAudio {
    pub data: Vec<u8>,
    pub format: String,
    pub sample_rate: u32,
    pub channels: u8,
}

pub async fn convert_for_stt(
    input_data: &[u8],
    original_filename: &str,
    provider: SttProvider,
) -> Result<ConvertedAudio, AudioError> {
    // Determine input format from filename
    let _input_extension = get_file_extension(original_filename);
    
    info!("Converting {} ({} bytes) for {:?} provider", 
        original_filename, input_data.len(), provider);

    // Create temporary input file
    let mut input_temp = NamedTempFile::new()
        .map_err(|e| AudioError::TempFile(format!("Failed to create input temp file: {}", e)))?;
    
    input_temp.write_all(input_data)
        .map_err(|e| AudioError::TempFile(format!("Failed to write input data: {}", e)))?;
    
    let input_path = input_temp.path();

    // Determine output format and parameters based on STT provider
    let (output_format, sample_rate, channels, codec) = match provider {
        SttProvider::ElevenLabs => {
            // ElevenLabs requires PCM 16kHz mono
            ("pcm", 16000, 1, "pcm_s16le")
        }
        SttProvider::Whisper => {
            // Whisper accepts MP3, but let's use WAV for consistency
            ("wav", 16000, 1, "pcm_s16le")
        }
        SttProvider::Google => {
            // Google Cloud STT prefers FLAC or linear16
            ("flac", 16000, 1, "flac")
        }
    };

    // Create temporary output file
    let output_temp = NamedTempFile::new()
        .map_err(|e| AudioError::TempFile(format!("Failed to create output temp file: {}", e)))?;
    
    let output_path = output_temp.path();

    // Check if ffmpeg is available
    if !is_ffmpeg_available() {
        return Err(AudioError::FfmpegNotFound);
    }

    // Build ffmpeg command
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y") // Overwrite output file
        .arg("-hide_banner")
        .arg("-loglevel").arg("error")
        .arg("-i").arg(input_path)
        .arg("-acodec").arg(codec)
        .arg("-ar").arg(sample_rate.to_string())
        .arg("-ac").arg(channels.to_string());

    // Add format-specific options
    match provider {
        SttProvider::ElevenLabs => {
            // For PCM, we need raw format
            cmd.arg("-f").arg("s16le");
        }
        SttProvider::Whisper => {
            // Standard WAV format
            cmd.arg("-f").arg("wav");
        }
        SttProvider::Google => {
            // FLAC format
            cmd.arg("-f").arg("flac");
        }
    }

    cmd.arg(output_path);

    debug!("Running ffmpeg command: {:?}", cmd);

    // Execute ffmpeg
    let output = cmd.output()
        .map_err(|e| AudioError::ConversionFailed(format!("Failed to execute ffmpeg: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AudioError::ConversionFailed(format!("FFmpeg failed: {}", stderr)));
    }

    // Read the converted audio data
    let converted_data = fs::read(output_path)
        .map_err(|e| AudioError::ConversionFailed(format!("Failed to read converted file: {}", e)))?;

    info!("Successfully converted audio: {} bytes -> {} bytes", 
        input_data.len(), converted_data.len());

    Ok(ConvertedAudio {
        data: converted_data,
        format: output_format.to_string(),
        sample_rate,
        channels,
    })
}

fn get_file_extension(filename: &str) -> &str {
    filename.rsplit('.').next().unwrap_or("")
}

fn is_ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_file_extension() {
        assert_eq!(get_file_extension("test.mp3"), "mp3");
        assert_eq!(get_file_extension("voice.ogg"), "ogg");
        assert_eq!(get_file_extension("file.with.dots.wav"), "wav");
        assert_eq!(get_file_extension("noextension"), "");
    }

    #[test]
    fn test_ffmpeg_availability() {
        // This test will only pass if ffmpeg is installed
        println!("FFmpeg available: {}", is_ffmpeg_available());
    }
}