# Multi-stage build for minimal final image
# Stage 1: Build dependencies and tools
FROM rust:1.82-slim as builder

# Install system dependencies needed for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release && rm -rf src

# Copy the actual source code
COPY src ./src

# Build the application with optimizations for static linking
RUN cargo build --release

# Stage 2: Runtime preparation with ffmpeg
FROM debian:bookworm-slim as runtime-prep

# Install ffmpeg and other runtime dependencies
RUN apt-get update && apt-get install -y \
    ffmpeg \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Stage 3: Final minimal image
FROM debian:bookworm-slim

# Install only essential runtime dependencies
RUN apt-get update && apt-get install -y \
    ffmpeg \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create a non-root user
RUN useradd -r -u 1000 -m -d /app -s /bin/bash app

# Set working directory
WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/telegram-stt-bot ./telegram-stt-bot

# Make sure the binary is executable
RUN chmod +x ./telegram-stt-bot

# Change ownership to app user
RUN chown -R app:app /app

# Switch to non-root user
USER app

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD pgrep telegram-stt-bot || exit 1

# Expose port (though Telegram bots don't need incoming connections)
EXPOSE 8080

# Set environment variables for production
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Run the application
CMD ["./telegram-stt-bot"]