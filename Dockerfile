# Multi-stage build for minimal final image
FROM rust:1.91.1-slim AS builder

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
RUN cargo build && rm -rf src

# Copy the actual source code
COPY src ./src

# Build the release application for production
RUN cargo build --release

# Final runtime image
FROM rust:1.91.1-slim

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

# Copy the optimized release binary
COPY --from=builder /app/target/release/telegram-stt-bot ./telegram-stt-bot

# Make sure the binary is executable
RUN chmod +x ./telegram-stt-bot

# Change ownership to app user
RUN chown -R app:app /app

# Switch to non-root user
USER app

# Expose port for health check endpoint
EXPOSE 8080

# Set environment variables for production
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Run the application
CMD ["./telegram-stt-bot"]
