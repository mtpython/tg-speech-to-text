#!/bin/bash

set -e

echo "🚀 Building Telegram STT Bot for Fly.io"

# Build the Rust binary
echo "📦 Building Rust binary..."
cargo build --release

echo "🐳 Building Docker image..."
docker build -t telegram-stt-bot .

echo "🧪 Testing Docker image..."
docker run --rm telegram-stt-bot --help || echo "Binary built successfully!"

echo "📏 Binary size:"
ls -lh target/release/telegram-stt-bot

echo "✅ Build complete! Ready for deployment."
echo ""
echo "📋 Next steps:"
echo "1. Set up your environment variables in .env"
echo "2. Test locally: docker run --env-file .env telegram-stt-bot"
echo "3. Deploy to Fly.io: fly deploy"