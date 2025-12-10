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

# Start all services
docker compose up --build -d

# Access the application
open http://localhost:8000
```

**Admin credentials:**
- Username: `admin`
- Password: Auto-generated on first run (check container logs)

On first startup, Readur generates a secure admin password and displays it in the logs:
```
==============================================
  READUR ADMIN USER CREATED
==============================================

Username: admin
Password: [your-generated-password]

⚠️   SAVE THESE CREDENTIALS IMMEDIATELY!
⚠️   This password will not be shown again.
==============================================
```

View the logs with: `docker compose logs readur`

To reset the admin password later, run: `readur reset-admin-password`

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
JWT_SECRET=your-super-secret-jwt-key-change-this
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

JWT_SECRET=your-super-secret-jwt-key-change-this
SERVER_ADDRESS=0.0.0.0:8000
UPLOAD_PATH=./uploads
WATCH_FOLDER=./watch
ALLOWED_FILE_TYPES=pdf,png,jpg,jpeg,gif,bmp,tiff,txt,rtf,doc,docx
```

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