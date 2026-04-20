# --- Frontend build stage ---
FROM node:24-bookworm as frontend-builder

WORKDIR /frontend
COPY frontend/package*.json ./
RUN npm install
COPY frontend ./
RUN npm run build

# --- Backend build stage ---
FROM rust:1.95-bookworm as backend-builder

# Install system dependencies for OCR and PDF processing
RUN apt-get update && apt-get install -y \
    tesseract-ocr \
    tesseract-ocr-all \
    libtesseract-dev \
    libleptonica-dev \
    pkg-config \
    libclang-dev \
    clang \
    poppler-utils \
    ocrmypdf \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release

# --- Runtime stage ---
FROM debian:bookworm-slim

# Install runtime dependencies:
#   - tesseract-ocr + language packs for OCR engine
#   - ghostscript + python3-pip for installing ocrmypdf from PyPI (latest)
#   - poppler-utils for pdftotext
#   - unpaper + pngquant for ocrmypdf optional image preprocessing
RUN apt-get update && apt-get install -y \
    tesseract-ocr \
    tesseract-ocr-all \
    ca-certificates \
    poppler-utils \
    ghostscript \
    unpaper \
    pngquant \
    python3 \
    python3-pip \
    curl \
    # Legacy DOC file support (lightweight tools)
    antiword \
    catdoc \
    && pip3 install --no-cache-dir --break-system-packages ocrmypdf \
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
