# 🚀 Deployment Guide for Telegram STT Bot

## Prerequisites

1. **Install Fly.io CLI**: https://fly.io/docs/hands-on/install-flyctl/
2. **Create Fly.io account**: https://fly.io/app/sign-up
3. **Get Telegram Bot Token**: Create a bot with @BotFather
4. **Get STT API Keys**: Choose one or more:
   - OpenAI API Key (for Whisper)
   - ElevenLabs API Key  
   - Google Cloud Service Account JSON

## Setup Steps

### 1. Login to Fly.io
```bash
fly auth login
```

### 2. Create Fly.io App
```bash
fly apps create telegram-stt-bot-$(openssl rand -hex 4)
```

### 3. Set Environment Variables
```bash
# Required: Telegram Bot Token
fly secrets set TELEGRAM_BOT_TOKEN="your_bot_token_here"

# Required: STT Provider (whisper, elevenlabs, or google)
fly secrets set STT_PROVIDER="whisper"

# For OpenAI Whisper
fly secrets set OPENAI_API_KEY="your_openai_key"

# For ElevenLabs
fly secrets set ELEVENLABS_API_KEY="your_elevenlabs_key"

# For Google Cloud STT
fly secrets set GOOGLE_CREDENTIALS_JSON='{"type":"service_account",...}'

# Optional: Logging level
fly secrets set RUST_LOG="info"
```

### 4. Deploy
```bash
fly deploy
```

### 5. Monitor
```bash
# Check logs
fly logs

# Check status
fly status

# Scale (if needed)
fly scale count 1
```

## Local Testing

### 1. Create .env file
```bash
cp .env.example .env
# Edit .env with your actual values
```

### 2. Build and test
```bash
./build.sh
```

### 3. Run locally
```bash
# With Docker
docker run --env-file .env telegram-stt-bot

# Or directly
cargo run
```

## Configuration Options

### STT Providers

| Provider | Format Required | Features |
|----------|----------------|----------|
| **whisper** | MP3, OGG, WAV | High accuracy, supports many languages |
| **elevenlabs** | PCM 16kHz mono | Fast, low latency |
| **google** | FLAC, WAV | Speaker diarization, language hints |

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `TELEGRAM_BOT_TOKEN` | ✅ | Bot token from @BotFather |
| `STT_PROVIDER` | ✅ | `whisper`, `elevenlabs`, or `google` |
| `OPENAI_API_KEY` | ⚠️ | Required if using Whisper |
| `ELEVENLABS_API_KEY` | ⚠️ | Required if using ElevenLabs |
| `GOOGLE_CREDENTIALS_JSON` | ⚠️ | Required if using Google Cloud |
| `RUST_LOG` | ❌ | Log level (`debug`, `info`, `warn`, `error`) |

## Supported File Types

- **Voice Messages**: Opus-encoded .ogg files
- **Audio Files**: .mp3, .m4a, .wav, .flac
- **Video Messages**: .mp4, .webm (audio extracted)
- **Documents**: Any audio/video file uploaded as document

## Health Checks

The bot exposes health endpoints:
- `GET /health` - Health check
- `GET /metrics` - Basic metrics

## Scaling

The bot is designed to scale efficiently on Fly.io:
- **Auto-scaling**: Scales down to 0 when not in use
- **Memory**: Uses ~50MB RAM base + processing overhead
- **CPU**: Minimal CPU usage except during transcription
- **Storage**: Temporary files are automatically cleaned up

## Troubleshooting

### Common Issues

1. **Bot not responding**
   - Check `fly logs` for errors
   - Verify `TELEGRAM_BOT_TOKEN` is correct
   - Ensure bot is not running elsewhere

2. **STT errors**
   - Verify API keys are correct
   - Check if provider supports the audio format
   - Monitor API rate limits

3. **File download failures**
   - Check file size limits (Telegram: 50MB)
   - Verify network connectivity

### Debug Commands

```bash
# Check environment variables
fly ssh console -C "env | grep -E '(TELEGRAM|STT|OPENAI|ELEVENLABS|GOOGLE)'"

# Check logs in real-time
fly logs -f

# Restart app
fly deploy --force
```

## Security Notes

- All API keys are stored as Fly.io secrets
- Temporary audio files are deleted after processing
- No user data is persisted
- All communication uses HTTPS/TLS

## Cost Estimation

### Fly.io Costs
- **Compute**: ~$0-5/month (auto-scaling to zero)
- **Network**: Minimal for audio downloads

### STT API Costs
- **OpenAI Whisper**: $0.006 per minute of audio
- **ElevenLabs**: Varies by plan
- **Google Cloud**: $0.004-0.016 per 15 seconds

## Support

- **Issues**: Create GitHub issues for bugs
- **Documentation**: See README.md
- **Telegram**: Test your bot by sending voice messages