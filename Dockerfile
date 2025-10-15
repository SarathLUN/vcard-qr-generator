# Multi-stage build for vCard QR Generator
# Stage 1: Build the application
FROM rust:latest AS builder

WORKDIR /app

# Install dependencies for SQLite
RUN apt-get update && \
    apt-get install -y \
    libsqlite3-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY migrations ./migrations
COPY static ./static

# Build the application in release mode
RUN cargo build --release --bin vcard-qr-generator

# Stage 2: Runtime image
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    libsqlite3-0 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1000 appuser && \
    chown -R appuser:appuser /app

# Copy the binary from builder
COPY --from=builder /app/target/release/vcard-qr-generator /usr/local/bin/vcard-qr-generator

# Copy static files and migrations
COPY --from=builder /app/static /app/static
COPY --from=builder /app/migrations /app/migrations

# Create directory for database with proper permissions
RUN mkdir -p /app/data && chown -R appuser:appuser /app/data

# Switch to non-root user
USER appuser

# Expose port
EXPOSE 3000

# Set environment variables
ENV RUST_LOG=info
ENV DATABASE_PATH=/app/data/vcards.db

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/login || exit 1

# Run the application
CMD ["vcard-qr-generator"]
