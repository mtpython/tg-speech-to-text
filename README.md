# Telegram Speech-to-Text Bot

Rust Telegram bot that transcribes voice messages, audio, and video using pluggable STT providers (Deepgram, OpenAI Whisper, ElevenLabs, Google Cloud).

See [RASPBERRY_PI_SETUP.md](RASPBERRY_PI_SETUP.md) for Docker deployment on a Raspberry Pi 4.

## Supported Inputs

- Voice messages (Opus/OGG)
- Audio files (MP3, M4A, WAV, OGG)
- Video files (MP4, WebM, AVI) — audio track is extracted via FFmpeg

## Prerequisites

- Rust 1.91.1+
- FFmpeg (bundled in the Docker image)
- Telegram bot token from [@BotFather](https://t.me/botfather)
- API key for one STT provider

## Configuration

Copy `.env.example` to `.env` and fill in:

| Variable | Required | Description |
|---|---|---|
| `TELEGRAM_BOT_TOKEN` | yes | Bot token from BotFather |
| `STT_PROVIDER` | no | `deepgram` (default), `whisper`, `elevenlabs`, `google` |
| `DEEPGRAM_API_KEY` | if used | Deepgram key (Nova-3 model) |
| `OPENAI_API_KEY` | if used | OpenAI key for Whisper |
| `ELEVENLABS_API_KEY` | if used | ElevenLabs key |
| `GOOGLE_CREDENTIALS_JSON` | if used | Service account JSON on a single line |
| `BOT_PASSWORD` | no | If set, users must authenticate before use |
| `ADMIN_USER_IDS` | no | Comma-separated Telegram user IDs allowed to run `/setprovider` |
| `RUST_LOG` | no | `error`, `warn`, `info` (default), `debug`, `trace` |

## Run Locally

```bash
cargo run --release
```

## Bot Commands

- `/start` — welcome
- `/help` — command list
- `/status` — bot status and configuration
- `/queue` — queue size and stats
- `/credits` — credit/balance/usage
- `/provider` — show current STT provider
- `/setprovider <name>` — switch provider (admin only)

## Project Structure

```
src/
├── main.rs           # entry point
├── handlers.rs       # Telegram message + command handlers
├── queue.rs          # processing queue
├── persistence.rs    # on-disk state
├── audio/convert.rs  # FFmpeg conversion
└── stt/
    ├── mod.rs
    ├── deepgram.rs
    ├── whisper.rs
    ├── elevenlabs.rs
    └── google.rs
```

Adding a new provider: create a module in `src/stt/`, implement `transcribe()`, and wire it into `SttProvider` in `src/stt/mod.rs`.

## License

MIT
