# Build stage
FROM rust:1.86-slim as builder

WORKDIR /app

# Copy manifest files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy src/main.rs to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source code
COPY src ./src

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/chess-server /app/chess-server

# Copy static files
COPY static ./static

# Expose port
EXPOSE 8000

# Run the application
CMD ["./chess-server"]

