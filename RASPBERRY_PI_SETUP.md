# 🥧 Raspberry Pi 4 Docker Setup Guide

Complete setup guide for running the Telegram Speech-to-Text Bot on Raspberry Pi 4 using Docker Compose.

## 📋 Prerequisites

- **Raspberry Pi 4** (4GB+ RAM recommended)
- **Raspberry Pi OS 64-bit** (Bullseye or newer)
- **Docker** and **Docker Compose** installed
- **Internet connection**
- **Telegram Bot Token** from [@BotFather](https://t.me/botfather)
- **STT Provider API Key** (OpenAI, ElevenLabs, or Google Cloud)

## 🐳 Step 1: Install Docker

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh

# Add pi user to docker group
sudo usermod -aG docker pi

# Docker Compose is included with Docker

# Reboot to apply group changes
sudo reboot
```

After reboot, verify installation:
```bash
docker --version
docker compose version
```

## 🚀 Step 2: Deploy the Bot

### Clone and Setup

```bash
# Clone the repository
cd /home/pi
git clone <your-repository-url> telegram-stt-bot
cd telegram-stt-bot

# Create environment file
cp .env.example .env
```

### Configure Environment

Edit the `.env` file with your credentials:
```bash
nano .env
```

**Required Settings:**
```bash
TELEGRAM_BOT_TOKEN=123456789:ABCdefGHIjklMNOpqrsTUVwxyz
STT_PROVIDER=whisper
OPENAI_API_KEY=sk-your_openai_key_here
```

**Optional Settings:**
```bash
BOT_PASSWORD=your_secure_password
RUST_LOG=info
```

### Build and Start

```bash
# Build the Docker image (ARM64 optimized)
docker compose build

# Start the service
docker compose up -d

# Check status
docker compose ps
docker compose logs -f telegram-stt-bot
```

## 📊 Step 3: Verify Installation

### Check Service Status
```bash
# Container status
docker compose ps

# View logs
docker compose logs telegram-stt-bot

# Health check
curl http://localhost:8080/health
```

### Test the Bot
1. Send `/start` to your bot on Telegram
2. Send a voice message or audio file
3. Check logs: `docker compose logs -f`

## 🔧 Management Commands

### Daily Operations
```bash
# View logs (live)
docker compose logs -f telegram-stt-bot

# Restart service
docker compose restart telegram-stt-bot

# Stop service
docker compose stop

# Start service
docker compose start

# Update and restart
git pull
docker compose build
docker compose up -d
```

### Resource Monitoring
```bash
# Container resource usage
docker stats telegram-stt-bot

# Pi system resources
htop
free -h
df -h
```

## 🔄 Step 4: Auto-Start on Boot

Create systemd service for Docker Compose:

```bash
sudo nano /etc/systemd/system/telegram-stt-bot.service
```

```ini
[Unit]
Description=Telegram STT Bot
After=docker.service
Requires=docker.service

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=/home/pi/telegram-stt-bot
ExecStart=/usr/bin/docker compose up -d
ExecStop=/usr/bin/docker compose down
User=pi

[Install]
WantedBy=multi-user.target
```

Enable auto-start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable telegram-stt-bot
sudo systemctl start telegram-stt-bot
```

## 🔍 Troubleshooting

### Common Issues

**1. Build fails on Pi 4:**
```bash
# Check architecture
uname -m  # Should show aarch64

# Clean and rebuild
docker compose down
docker system prune -f
docker compose build --no-cache
```

**2. Out of memory during build:**
```bash
# Increase swap space
sudo dphys-swapfile swapoff
sudo nano /etc/dphys-swapfile
# Set CONF_SWAPSIZE=2048
sudo dphys-swapfile setup
sudo dphys-swapfile swapon
```

**3. Bot not responding:**
```bash
# Check logs
docker compose logs telegram-stt-bot

# Verify environment
docker compose exec telegram-stt-bot env | grep TELEGRAM

# Test network
curl -s https://api.telegram.org/bot<TOKEN>/getMe
```

**4. Audio processing fails:**
```bash
# Check FFmpeg in container
docker compose exec telegram-stt-bot ffmpeg -version

# Test STT API
docker compose logs | grep -i "stt\|error"
```

### Performance Optimization

**Docker System Monitoring:**
```bash
# Container resource usage
docker stats telegram-stt-bot

# System resources
free -h && df -h
```

**Storage Cleanup:**
```bash
# Clean Docker data
docker system prune -f
docker volume prune -f
docker image prune -f

# Clean logs
sudo journalctl --vacuum-time=7d
```

## 📈 Monitoring

### Health Checks
```bash
# HTTP health endpoint
curl http://localhost:8080/health

# Container health
docker compose ps telegram-stt-bot
```

### Queue Statistics
Send `/queue` to your bot to see:
- Current queue size
- Items processed
- Processing status

## 🔒 Security Notes

- Bot runs as non-root user in container
- Temporary files isolated with tmpfs
- No unnecessary capabilities granted
- Secrets managed via environment variables
- Network isolation with custom bridge

## 🆙 Updates

```bash
# Update code
cd /home/pi/telegram-stt-bot
git pull

# Rebuild and restart
docker compose build
docker compose up -d

# Cleanup old images
docker image prune -f
```

## 📧 Support

- **Logs**: `docker compose logs telegram-stt-bot`
- **Health**: `curl http://localhost:8080/health`
- **Queue Status**: Send `/queue` to bot
- **System Status**: Send `/status` to bot

---

🎯 **Your Pi 4 is now running a production-ready Telegram STT bot with Docker!**