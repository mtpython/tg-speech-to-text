# Raspberry Pi 4 Deployment

Runs the bot in Docker Compose on Raspberry Pi OS 64-bit (Bullseye+).

## Prerequisites

- Raspberry Pi 4 (4 GB+ RAM recommended)
- Raspberry Pi OS 64-bit
- Telegram bot token and one STT provider API key

## 1. Install Docker

```bash
sudo apt update && sudo apt upgrade -y
curl -fsSL https://get.docker.com | sudo sh
sudo usermod -aG docker pi
sudo reboot
```

Verify after reboot:

```bash
docker --version
docker compose version
```

## 2. Deploy

```bash
cd /home/pi
git clone <your-repository-url> telegram-stt-bot
cd telegram-stt-bot
cp .env.example .env
nano .env   # fill in TELEGRAM_BOT_TOKEN, STT_PROVIDER, provider API key
docker compose up -d --build
```

Check status:

```bash
docker compose ps
docker compose logs -f telegram-stt-bot
curl http://localhost:8091/health
```

## 3. Auto-Start on Boot

```bash
sudo tee /etc/systemd/system/telegram-stt-bot.service >/dev/null <<'EOF'
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
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now telegram-stt-bot
```

## Common Operations

```bash
docker compose logs -f telegram-stt-bot   # live logs
docker compose restart telegram-stt-bot   # restart
docker compose down && docker compose up -d --build   # update after git pull
docker stats telegram-stt-bot             # resource usage
docker system prune -f                    # reclaim disk
```

## Troubleshooting

**Build runs out of memory.** Increase swap to 2 GB:

```bash
sudo dphys-swapfile swapoff
sudo sed -i 's/^CONF_SWAPSIZE=.*/CONF_SWAPSIZE=2048/' /etc/dphys-swapfile
sudo dphys-swapfile setup && sudo dphys-swapfile swapon
```

**Bot not responding.** Check logs and verify the token:

```bash
docker compose logs telegram-stt-bot
curl -s "https://api.telegram.org/bot<TOKEN>/getMe"
```

**Audio processing fails.** Verify FFmpeg and look for STT errors:

```bash
docker compose exec telegram-stt-bot ffmpeg -version
docker compose logs | grep -iE "stt|error"
```

## Health & Status

- `curl http://localhost:8091/health` — HTTP health endpoint
- `/status` in Telegram — bot configuration
- `/queue` in Telegram — queue size and stats
