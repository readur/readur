# Installation Guide

This guide covers various methods to install and run Readur, from quick Docker deployment to manual installation.

## Table of Contents

- [Quick Start with Docker Compose](#quick-start-with-docker-compose)
- [System Requirements](#system-requirements)
- [Manual Installation](#manual-installation)
  - [Prerequisites](#prerequisites)
  - [Backend Setup](#backend-setup)
  - [Frontend Setup](#frontend-setup)
- [Verifying Installation](#verifying-installation)

## Quick Start with Docker Compose

The fastest way to get Readur running:

```bash
# Clone the repository
git clone https://github.com/perfectra1n/readur
cd readur

# Optional: JWT_SECRET signs session tokens. When unset, a key is generated on
# first start and stored in the database. To manage it yourself:
# echo "JWT_SECRET=$(openssl rand -hex 32)" > .env

# Start all services
docker compose up --build -d

# Access the application
open http://localhost:8000
```

**Admin credentials:**
- Username: `admin` (override with `ADMIN_USERNAME`)
- Password: the value of `ADMIN_PASSWORD` if set; otherwise generated on first run and written to a file

If `ADMIN_PASSWORD` is not set, Readur generates a random 24-character password on first startup and writes it to `initial-admin-password` in the `.readur` directory inside `UPLOAD_PATH` (`/app/uploads/.readur/initial-admin-password` in the Docker image) with mode `0600`. The password is not written to the logs; the startup log shows the file path. Set `ADMIN_PASSWORD_FILE` to choose a different location.

```bash
docker compose exec readur cat /app/uploads/.readur/initial-admin-password
```

Sign in, change the password, then delete the file.

To reset the admin password later:
```bash
docker compose exec readur readur reset-admin-password
```

### What You Get

After deployment, you'll have:
- **Web Interface**: Modern document management UI at `http://localhost:8000`
- **PostgreSQL Database**: Document metadata and full-text search indexes
- **File Storage**: Persistent document storage with OCR processing
- **Watch Folder**: Automatic file ingestion from mounted directories
- **REST API**: Full API access for integrations

## System Requirements

### Minimum Requirements
- **CPU**: 2 cores
- **RAM**: 2GB
- **Storage**: 10GB free space
- **OS**: Linux, macOS, or Windows with Docker

### Recommended for Production
- **CPU**: 4+ cores
- **RAM**: 4GB+
- **Storage**: 50GB+ SSD
- **Network**: Stable internet connection for OCR processing

## Manual Installation

For development or custom deployments without Docker:

### Prerequisites

Install these dependencies on your system:

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y \
    tesseract-ocr tesseract-ocr-eng \
    libtesseract-dev libleptonica-dev \
    postgresql postgresql-contrib \
    pkg-config libclang-dev

# macOS (requires Homebrew)
brew install tesseract leptonica postgresql rust nodejs npm

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Backend Setup

1. **Configure Database**:
```bash
# Create database and user
sudo -u postgres psql
CREATE DATABASE readur;
CREATE USER readur_user WITH ENCRYPTED PASSWORD 'your_password';
GRANT ALL PRIVILEGES ON DATABASE readur TO readur_user;
\q
```

2. **Environment Configuration**:
```bash
# Copy environment template
cp .env.example .env

# Edit configuration
nano .env
```

Required environment variables:

**Option 1: Using DATABASE_URL (recommended)**:
```env
DATABASE_URL=postgresql://readur_user:your_password@localhost/readur
JWT_SECRET=<output of: openssl rand -hex 32>
SERVER_ADDRESS=0.0.0.0:8000
UPLOAD_PATH=./uploads
WATCH_FOLDER=./watch
ALLOWED_FILE_TYPES=pdf,png,jpg,jpeg,gif,bmp,tiff,txt,rtf,doc,docx
```

**Option 2: Using individual PostgreSQL variables**:
```env
# DATABASE_URL takes priority if set, but you can use individual variables instead
POSTGRES_HOST=localhost
POSTGRES_PORT=5432
POSTGRES_DB=readur
POSTGRES_USER=readur_user
POSTGRES_PASSWORD=your_password

JWT_SECRET=<output of: openssl rand -hex 32>
SERVER_ADDRESS=0.0.0.0:8000
UPLOAD_PATH=./uploads
WATCH_FOLDER=./watch
ALLOWED_FILE_TYPES=pdf,png,jpg,jpeg,gif,bmp,tiff,txt,rtf,doc,docx
```

> **Note**: `JWT_SECRET` is optional. When it is unset, Readur generates a signing key on first start and stores it in the database; `readur rotate-jwt-secret` replaces it. If you set `JWT_SECRET`, it must be at least 32 bytes and not a published example value (generate one with `openssl rand -hex 32`), and it takes precedence over the stored key.

> **Note**: If `DATABASE_URL` is set, it takes priority over individual PostgreSQL variables. This is useful for different deployment scenarios where some platforms provide a single connection string while others provide individual components.

3. **Build and Run Backend**:
```bash
# Install dependencies and run
cargo build --release
cargo run
```

### Frontend Setup

1. **Install Dependencies**:
```bash
cd frontend
npm install
```

2. **Development Mode**:
```bash
npm run dev
# Frontend available at http://localhost:5173
```

3. **Production Build**:
```bash
npm run build
# Built files in frontend/dist/
```

### Resetting Admin Password

For manual installations, reset the admin password using:
```bash
./target/release/readur reset-admin-password
```

Or if running via `cargo run`:
```bash
cargo run -- reset-admin-password
```

## Verifying Installation

After installation, verify everything is working:

1. **Check Backend Health**:
```bash
curl http://localhost:8000/api/health
```

2. **Access Web Interface**:
   - Navigate to `http://localhost:8000`
   - Log in with default credentials
   - Upload a test document

3. **Verify Database Connection**:
```bash
# For Docker installation
docker exec -it readur-postgres-1 psql -U readur -c "\dt"

# For manual installation
psql -U readur_user -d readur -c "\dt"
```

4. **Check OCR Functionality**:
   - Upload a PDF or image file
   - Wait for processing to complete
   - Search for text content from the uploaded file

## Next Steps

- [Configure Readur](configuration.md) for your specific needs
- Set up [production deployment](deployment.md) with SSL and proper security
- Read the [User Guide](user-guide.md) to learn about all features
- Explore the [API Reference](api-reference.md) for integrations