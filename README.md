# 🎤 Telegram Speech-to-Text Bot

A high-performance Rust Telegram bot that transcribes voice messages, audio files, and video files using multiple STT providers. Optimized for deployment on Fly.io with minimal resource usage and fast startup.

## ✨ Features

- 🎵 **Multi-format Audio Support**: Voice messages (Opus/OGG), audio files (MP3, M4A, WAV), and video files (MP4, WebM)
- 🧠 **Multiple STT Providers**: OpenAI Whisper, ElevenLabs STT, and Google Cloud Speech-to-Text
- 🚀 **Fly.io Optimized**: Minimal Docker image, statically compiled binary, auto-scaling
- 🔄 **Format Conversion**: Automatic audio conversion using FFmpeg
- 📊 **Comprehensive Logging**: Detailed logging with performance metrics
- 🛡️ **Error Handling**: Graceful error handling with user-friendly messages
- 💾 **Memory Efficient**: Automatic cleanup of temporary files

## 🏗️ Architecture

```
┌─────────────────┐    ┌──────────────┐    ┌─────────────────┐
│   Telegram      │───▶│   Bot Core   │───▶│   STT Provider  │
│   API           │    │   (Rust)     │    │   (API)         │
└─────────────────┘    └──────────────┘    └─────────────────┘
                              │
                              ▼
                       ┌──────────────┐
                       │   FFmpeg     │
                       │   (Audio     │
                       │   Conversion)│
                       └──────────────┘
```

## 📋 Prerequisites

- Rust 1.91.1+ (for development)
- Docker (for deployment)
- FFmpeg (included in Docker image)
- Telegram Bot Token from [@BotFather](https://t.me/botfather)
- API key from one of the supported STT providers

## 🚀 Quick Start

### 1. Create Telegram Bot

1. Message [@BotFather](https://t.me/botfather) on Telegram
2. Create a new bot with `/newbot`
3. Save the bot token

### 2. Get STT Provider API Key

Choose one of the following:

**OpenAI Whisper (Recommended)**
- Sign up at [OpenAI](https://platform.openai.com/)
- Generate an API key
- Set `STT_PROVIDER=whisper`

**ElevenLabs STT**
- Sign up at [ElevenLabs](https://elevenlabs.io/)
- Generate an API key
- Set `STT_PROVIDER=elevenlabs`

**Google Cloud STT**
- Create a Google Cloud project
- Enable Speech-to-Text API
- Create service account credentials
- Set `STT_PROVIDER=google`

### 3. Local Development

```bash
# Clone and setup
git clone <repository>
cd telegram-stt-bot

# Install dependencies (if building locally)
cargo build --release

# Configure environment
cp .env.example .env
# Edit .env with your credentials

# Run locally
cargo run
```

### 4. Deploy to Fly.io

```bash
# Install Fly CLI
curl -L https://fly.io/install.sh | sh

# Login to Fly.io
flyctl auth login

# Deploy the app
flyctl launch --copy-config --yes

# Set secrets
flyctl secrets set TELEGRAM_BOT_TOKEN="your_token_here"
flyctl secrets set STT_PROVIDER="whisper"
flyctl secrets set OPENAI_API_KEY="your_openai_key_here"

# Deploy
flyctl deploy
```

## 🔧 Configuration

### Environment Variables

| Variable | Required | Description | Example |
|----------|----------|-------------|---------|
| `TELEGRAM_BOT_TOKEN` | ✅ | Bot token from BotFather | `123456789:ABCdefGHIjkl...` |
| `STT_PROVIDER` | ✅ | STT provider to use | `whisper`, `elevenlabs`, `google` |
| `OPENAI_API_KEY` | 🔶 | OpenAI API key (if using Whisper) | `sk-...` |
| `ELEVENLABS_API_KEY` | 🔶 | ElevenLabs API key | `el_...` |
| `GOOGLE_CREDENTIALS_JSON` | 🔶 | Google service account JSON | `{"type":"service_account",...}` |
| `RUST_LOG` | ❌ | Log level | `info`, `debug`, `warn` |

🔶 = Required for specific STT provider

### STT Provider Comparison

| Provider | Accuracy | Speed | Cost | Languages | Notes |
|----------|----------|-------|------|-----------|-------|
| **OpenAI Whisper** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ | 90+ | Best overall choice |
| **ElevenLabs** | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐ | English | Fastest, good for real-time |
| **Google Cloud** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ | 120+ | Enterprise features |

## 🤖 Bot Commands

- `/start` - Welcome message and instructions
- `/help` - Show available commands
- `/status` - Display bot status and configuration

## 📝 Usage

1. **Voice Messages**: Record and send voice messages directly in Telegram
2. **Audio Files**: Send audio files (.mp3, .m4a, .ogg, .wav, etc.)
3. **Video Files**: Send video files (.mp4, .webm, .avi) - audio will be extracted
4. **File Upload**: Use the attachment button to upload audio/video files

The bot will:
1. ✅ Acknowledge your message
2. 🎵 Show "Processing audio..." status
3. 🔄 Convert audio to the required format
4. 🧠 Send to STT provider for transcription
5. 📝 Reply with the transcribed text

## 🐳 Docker

### Build Locally

```bash
docker build -t telegram-stt-bot .
docker run -d --env-file .env telegram-stt-bot
```

### Multi-arch Build

```bash
docker buildx build --platform linux/amd64,linux/arm64 -t telegram-stt-bot .
```

## 📊 Performance

### Resource Usage
- **Memory**: ~50MB base + ~100MB per concurrent transcription
- **CPU**: Low usage, spikes during audio conversion
- **Disk**: Temporary files cleaned automatically
- **Network**: Bandwidth depends on audio file sizes

### Benchmarks
- **Voice Message (10s)**: ~2-5 seconds end-to-end
- **Audio File (1MB)**: ~3-8 seconds end-to-end
- **Cold Start**: ~500ms on Fly.io

## 🔍 Troubleshooting

### Common Issues

**Bot not responding**
```bash
# Check logs
flyctl logs

# Check if bot is running
flyctl status
```

**Audio conversion fails**
```bash
# Verify FFmpeg is available
docker run telegram-stt-bot ffmpeg -version
```

**STT API errors**
- Check API key validity
- Verify account has sufficient credits
- Check rate limits

### Debug Mode

```bash
# Enable debug logging
flyctl secrets set RUST_LOG="debug"
flyctl deploy
```

## 🛠️ Development

### Project Structure

```
src/
├── main.rs              # Application entry point
├── handlers.rs          # Telegram message handlers
├── audio/
│   ├── mod.rs          # Audio module exports
│   └── convert.rs      # FFmpeg audio conversion
└── stt/
    ├── mod.rs          # STT provider interface
    ├── whisper.rs      # OpenAI Whisper integration
    ├── elevenlabs.rs   # ElevenLabs integration
    └── google.rs       # Google Cloud STT integration
```

### Adding New STT Providers

1. Create new module in `src/stt/`
2. Implement `transcribe()` function
3. Add provider to `SttProvider` enum
4. Update configuration handling

### Running Tests

```bash
cargo test
```

## 📈 Monitoring

### Metrics Available
- Request count by STT provider
- Response times
- Error rates
- Memory usage

### Fly.io Monitoring

```bash
# View logs
flyctl logs

# Check resource usage
flyctl metrics

# Scale manually if needed
flyctl scale count 2
```

## 🔒 Security

- Non-root user in Docker container
- Secrets managed via Fly.io secrets
- No persistent storage of audio files
- HTTPS enforced for all connections

## 🤝 Contributing

1. Fork the repository
2. Create feature branch (`git checkout -b feature/amazing-feature`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feature/amazing-feature`)
5. Open Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙋‍♂️ Support

- 📧 Create an issue for bug reports
- 💬 Discussions for feature requests
- 📖 Check the documentation for common questions

## 🔗 Links

- [Fly.io Documentation](https://fly.io/docs/)
- [Teloxide Documentation](https://docs.rs/teloxide/)
- [OpenAI Whisper API](https://platform.openai.com/docs/guides/speech-to-text)
- [ElevenLabs STT API](https://elevenlabs.io/docs/speech-to-text)
- [Google Cloud STT](https://cloud.google.com/speech-to-text)

---

Made with ❤️ and 🦀 Rust
