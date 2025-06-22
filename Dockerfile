# --- Frontend build stage ---
FROM node:22-bookworm as frontend-builder

WORKDIR /frontend
COPY frontend/package*.json ./
RUN npm install
COPY frontend ./
RUN npm run build

# --- Backend build stage ---
FROM rust:1.85-bookworm as backend-builder

# Install system dependencies for OCR
RUN apt-get update && apt-get install -y \
    tesseract-ocr \
    tesseract-ocr-eng \
    libtesseract-dev \
    libleptonica-dev \
    pkg-config \
    libclang-dev \
    clang \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release

# --- Runtime stage ---
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    tesseract-ocr \
    tesseract-ocr-eng \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy backend binary
COPY --from=backend-builder /app/target/release/readur /app/readur

# Copy migrations directory
COPY --from=backend-builder /app/migrations /app/migrations

# Create necessary directories
RUN mkdir -p /app/uploads /app/watch /app/frontend

# Set permissions for watch folder to handle various mount scenarios
RUN chmod 755 /app/watch

# Copy built frontend from frontend-builder
COPY --from=frontend-builder /frontend/dist /app/frontend/dist

EXPOSE 8000

CMD ["./readur"]
