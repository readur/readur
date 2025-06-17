# Readur 📄

A powerful, modern document management system built with Rust and React. Readur provides intelligent document processing with OCR capabilities, full-text search, and a beautiful web interface designed for 2026 tech standards.

## ✨ Features

- 🔐 **Secure Authentication**: JWT-based user authentication with bcrypt password hashing
- 📤 **Smart File Upload**: Drag-and-drop support for PDF, images, text files, and Office documents
- 🔍 **Advanced OCR**: Automatic text extraction using Tesseract for searchable document content
- 🔎 **Powerful Search**: PostgreSQL full-text search with advanced filtering and ranking
- 👁️ **Folder Monitoring**: Non-destructive file watching (unlike paperless-ngx, doesn't consume source files)
- 🎨 **Modern UI**: Beautiful React frontend with Material-UI components and responsive design
- 🐳 **Docker Ready**: Complete containerization with production-ready multi-stage builds
- ⚡ **High Performance**: Rust backend for speed and reliability
- 📊 **Analytics Dashboard**: Document statistics and processing status overview

## 🚀 Quick Start

### Using Docker Compose (Recommended)

The fastest way to get Readur running:

```bash
# Clone the repository
git clone <repository-url>
cd readur

# Start all services
docker compose up --build -d

# Access the application
open http://localhost:8000
```

**Default login credentials:**
- Username: `admin`
- Password: `readur2024`

> ⚠️ **Important**: Change the default admin password immediately after first login!

### What You Get

After deployment, you'll have:
- **Web Interface**: Modern document management UI at `http://localhost:8000`
- **PostgreSQL Database**: Document metadata and full-text search indexes
- **File Storage**: Persistent document storage with OCR processing
- **Watch Folder**: Automatic file ingestion from mounted directories
- **REST API**: Full API access for integrations

## 🐳 Docker Deployment Guide

### Production Docker Compose

For production deployments, create a custom `docker-compose.prod.yml`:

```yaml
services:
  readur:
    image: readur:latest
    ports:
      - "8000:8000"
    environment:
      # Core Configuration
      - DATABASE_URL=postgresql://readur:${DB_PASSWORD}@postgres:5432/readur
      - JWT_SECRET=${JWT_SECRET}
      - SERVER_ADDRESS=0.0.0.0:8000
      
      # File Storage
      - UPLOAD_PATH=/app/uploads
      - WATCH_FOLDER=/app/watch
      - ALLOWED_FILE_TYPES=pdf,png,jpg,jpeg,tiff,bmp,gif,txt,doc,docx
      
      # Watch Folder Settings
      - WATCH_INTERVAL_SECONDS=30
      - FILE_STABILITY_CHECK_MS=500
      - MAX_FILE_AGE_HOURS=168
      
      # OCR Configuration
      - OCR_LANGUAGE=eng
      - CONCURRENT_OCR_JOBS=4
      - OCR_TIMEOUT_SECONDS=300
      - MAX_FILE_SIZE_MB=100
      
      # Performance Tuning
      - MEMORY_LIMIT_MB=1024
      - CPU_PRIORITY=normal
      - ENABLE_COMPRESSION=true
    
    volumes:
      # Document storage
      - ./data/uploads:/app/uploads
      
      # Watch folder - mount your network drives here
      - /mnt/nfs/documents:/app/watch
      # or SMB: - /mnt/smb/shared:/app/watch
      # or S3: - /mnt/s3/bucket:/app/watch
    
    depends_on:
      - postgres
    restart: unless-stopped
    
    # Resource limits for production
    deploy:
      resources:
        limits:
          memory: 2G
          cpus: '2.0'
        reservations:
          memory: 512M
          cpus: '0.5'

  postgres:
    image: postgres:15
    environment:
      - POSTGRES_USER=readur
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=readur
      - POSTGRES_INITDB_ARGS=--encoding=UTF-8 --lc-collate=en_US.UTF-8 --lc-ctype=en_US.UTF-8
    
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./postgres-config:/etc/postgresql/conf.d:ro
    
    # PostgreSQL optimization for document search
    command: >
      postgres
      -c shared_buffers=256MB
      -c effective_cache_size=1GB
      -c max_connections=100
      -c default_text_search_config=pg_catalog.english
    
    restart: unless-stopped
    
    # Don't expose port in production
    # ports:
    #   - "5433:5432"

volumes:
  postgres_data:
    driver: local
```

### Environment Variables

#### Port Configuration

Readur supports flexible port configuration:

```bash
# Method 1: Specify full server address
SERVER_ADDRESS=0.0.0.0:8000

# Method 2: Use separate host and port (recommended)
SERVER_HOST=0.0.0.0
SERVER_PORT=8000

# For development: Configure frontend port
CLIENT_PORT=5173
BACKEND_PORT=8000
```

#### Security Configuration

Create a `.env` file for your secrets:

```bash
# Generate secure secrets
JWT_SECRET=$(openssl rand -base64 64)
DB_PASSWORD=$(openssl rand -base64 32)

# Save to .env file
cat > .env << EOF
JWT_SECRET=${JWT_SECRET}
DB_PASSWORD=${DB_PASSWORD}
EOF
```

Deploy with:
```bash
docker compose -f docker-compose.prod.yml --env-file .env up -d
```

### Network Filesystem Mounts

#### NFS Mounts
```bash
# Mount NFS share
sudo mount -t nfs 192.168.1.100:/documents /mnt/nfs/documents

# Add to docker-compose.yml
volumes:
  - /mnt/nfs/documents:/app/watch
environment:
  - WATCH_INTERVAL_SECONDS=60
  - FILE_STABILITY_CHECK_MS=1000
  - FORCE_POLLING_WATCH=1
```

#### SMB/CIFS Mounts
```bash
# Mount SMB share
sudo mount -t cifs //server/share /mnt/smb/shared -o username=user,password=pass

# Docker volume configuration
volumes:
  - /mnt/smb/shared:/app/watch
environment:
  - WATCH_INTERVAL_SECONDS=30
  - FILE_STABILITY_CHECK_MS=2000
```

#### S3 Mounts (using s3fs)
```bash
# Mount S3 bucket
s3fs mybucket /mnt/s3/bucket -o passwd_file=~/.passwd-s3fs

# Docker configuration for S3
volumes:
  - /mnt/s3/bucket:/app/watch
environment:
  - WATCH_INTERVAL_SECONDS=120
  - FILE_STABILITY_CHECK_MS=5000
  - FORCE_POLLING_WATCH=1
```

### SSL/HTTPS Setup

Use a reverse proxy like Nginx or Traefik:

#### Nginx Configuration
```nginx
server {
    listen 443 ssl http2;
    server_name readur.yourdomain.com;
    
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    
    location / {
        proxy_pass http://localhost:8000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # For file uploads
        client_max_body_size 100M;
        proxy_read_timeout 300s;
        proxy_send_timeout 300s;
    }
}
```

#### Traefik Configuration
```yaml
services:
  readur:
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.readur.rule=Host(`readur.yourdomain.com`)"
      - "traefik.http.routers.readur.tls=true"
      - "traefik.http.routers.readur.tls.certresolver=letsencrypt"
```

> 📘 **For detailed reverse proxy configurations** including Apache, Caddy, custom ports, load balancing, and advanced scenarios, see [REVERSE_PROXY.md](./REVERSE_PROXY.md).

### Health Checks

Add health checks to your Docker configuration:

```yaml
services:
  readur:
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/api/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
```

### Backup Strategy

```bash
#!/bin/bash
# backup.sh - Automated backup script

# Backup database
docker exec readur-postgres-1 pg_dump -U readur readur | gzip > backup_$(date +%Y%m%d_%H%M%S).sql.gz

# Backup uploaded files
tar -czf uploads_backup_$(date +%Y%m%d_%H%M%S).tar.gz -C ./data uploads/

# Clean old backups (keep 30 days)
find . -name "backup_*.sql.gz" -mtime +30 -delete
find . -name "uploads_backup_*.tar.gz" -mtime +30 -delete
```

### Monitoring

Monitor your deployment with Docker stats:

```bash
# Real-time resource usage
docker stats

# Container logs
docker compose logs -f readur

# Watch folder activity
docker compose logs -f readur | grep watcher
```

## 🏗️ Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   React Frontend │────│   Rust Backend  │────│  PostgreSQL DB  │
│   (Port 8000)   │    │   (Axum API)    │    │   (Port 5433)   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         │              ┌─────────────────┐             │
         └──────────────│  File Storage   │─────────────┘
                        │  + OCR Engine   │
                        └─────────────────┘
```

## 📋 System Requirements

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

## 🛠️ Manual Installation

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
```env
DATABASE_URL=postgresql://readur_user:your_password@localhost/readur
JWT_SECRET=your-super-secret-jwt-key-change-this
SERVER_ADDRESS=0.0.0.0:8000
UPLOAD_PATH=./uploads
WATCH_FOLDER=./watch
ALLOWED_FILE_TYPES=pdf,png,jpg,jpeg,gif,bmp,tiff,txt,rtf,doc,docx
```

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

## 📖 User Guide

### Getting Started

1. **First Login**: Use the default admin credentials to access the system
2. **Upload Documents**: Drag and drop files or use the upload button
3. **Wait for Processing**: OCR processing happens automatically in the background
4. **Search and Organize**: Use the powerful search features to find your documents

### Supported File Types

| Type | Extensions | OCR Support | Notes |
|------|-----------|-------------|-------|
| **PDF** | `.pdf` | ✅ | Text extraction + OCR for scanned pages |
| **Images** | `.png`, `.jpg`, `.jpeg`, `.tiff`, `.bmp`, `.gif` | ✅ | Full OCR text extraction |
| **Text** | `.txt`, `.rtf` | ❌ | Direct text indexing |
| **Office** | `.doc`, `.docx` | ⚠️ | Limited support |

### Using the Interface

#### Dashboard
- **Document Statistics**: Total documents, storage usage, OCR status
- **Recent Activity**: Latest uploads and processing status
- **Quick Actions**: Fast access to upload and search

#### Document Management
- **List/Grid View**: Toggle between different viewing modes
- **Sorting**: Sort by date, name, size, or file type
- **Filtering**: Filter by tags, file types, and OCR status
- **Bulk Actions**: Select multiple documents for batch operations

#### Advanced Search
- **Full-text Search**: Search within document content
- **Metadata Filters**: Filter by upload date, file size, type
- **Tag System**: Organize documents with custom tags
- **OCR Status**: Find processed vs. pending documents

#### Folder Watching
- **Non-destructive**: Unlike paperless-ngx, source files remain untouched
- **Automatic Processing**: New files are detected and processed automatically
- **Configurable**: Set custom watch directories

### Tips for Best Results

1. **OCR Quality**: Higher resolution images (300+ DPI) produce better OCR results
2. **File Organization**: Use consistent naming conventions for easier searching
3. **Regular Backups**: Backup both database and file storage regularly
4. **Performance**: For large document collections, consider increasing server resources

## 🔧 Configuration

### Environment Variables

All application settings can be configured via environment variables:

#### Core Configuration
| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `postgresql://readur:readur@localhost/readur` | PostgreSQL connection string |
| `JWT_SECRET` | `your-secret-key` | Secret key for JWT tokens ⚠️ **Change in production!** |
| `SERVER_ADDRESS` | `0.0.0.0:8000` | Server bind address and port |

#### File Storage & Upload
| Variable | Default | Description |
|----------|---------|-------------|
| `UPLOAD_PATH` | `./uploads` | Document storage directory |
| `ALLOWED_FILE_TYPES` | `pdf,txt,doc,docx,png,jpg,jpeg` | Comma-separated allowed file extensions |

#### Watch Folder Configuration
| Variable | Default | Description |
|----------|---------|-------------|
| `WATCH_FOLDER` | `./watch` | Directory to monitor for new files |
| `WATCH_INTERVAL_SECONDS` | `30` | Polling interval for network filesystems (seconds) |
| `FILE_STABILITY_CHECK_MS` | `500` | Time to wait for file write completion (milliseconds) |
| `MAX_FILE_AGE_HOURS` | _(none)_ | Skip files older than this many hours |
| `FORCE_POLLING_WATCH` | _(none)_ | Force polling mode even for local filesystems |

#### OCR & Processing Settings
*Note: These settings can also be configured per-user via the web interface*

| Variable | Default | Description |
|----------|---------|-------------|
| `OCR_LANGUAGE` | `eng` | OCR language code (eng, fra, deu, spa, etc.) |
| `CONCURRENT_OCR_JOBS` | `4` | Maximum parallel OCR processes |
| `OCR_TIMEOUT_SECONDS` | `300` | OCR processing timeout per file |
| `MAX_FILE_SIZE_MB` | `50` | Maximum file size for processing |
| `AUTO_ROTATE_IMAGES` | `true` | Automatically rotate images for better OCR |
| `ENABLE_IMAGE_PREPROCESSING` | `true` | Apply image enhancement before OCR |

#### Search & Performance
| Variable | Default | Description |
|----------|---------|-------------|
| `SEARCH_RESULTS_PER_PAGE` | `25` | Default number of search results per page |
| `SEARCH_SNIPPET_LENGTH` | `200` | Length of text snippets in search results |
| `FUZZY_SEARCH_THRESHOLD` | `0.8` | Similarity threshold for fuzzy search (0.0-1.0) |
| `MEMORY_LIMIT_MB` | `512` | Memory limit for OCR processes |
| `CPU_PRIORITY` | `normal` | CPU priority: `low`, `normal`, `high` |

#### Data Management
| Variable | Default | Description |
|----------|---------|-------------|
| `RETENTION_DAYS` | _(none)_ | Auto-delete documents after N days |
| `ENABLE_AUTO_CLEANUP` | `false` | Enable automatic cleanup of old documents |
| `ENABLE_COMPRESSION` | `false` | Compress stored documents to save space |
| `ENABLE_BACKGROUND_OCR` | `true` | Process OCR in background queue |

### Example Production Configuration

```env
# Core settings
DATABASE_URL=postgresql://readur:secure_password@postgres:5432/readur
JWT_SECRET=your-very-long-random-secret-key-generated-with-openssl
SERVER_ADDRESS=0.0.0.0:8000

# File handling
UPLOAD_PATH=/app/uploads
ALLOWED_FILE_TYPES=pdf,png,jpg,jpeg,tiff,bmp,gif,txt,rtf,doc,docx

# Watch folder for NFS mount
WATCH_FOLDER=/mnt/nfs/documents
WATCH_INTERVAL_SECONDS=60
FILE_STABILITY_CHECK_MS=1000
MAX_FILE_AGE_HOURS=168
FORCE_POLLING_WATCH=1

# OCR optimization
OCR_LANGUAGE=eng
CONCURRENT_OCR_JOBS=8
OCR_TIMEOUT_SECONDS=600
MAX_FILE_SIZE_MB=200
AUTO_ROTATE_IMAGES=true
ENABLE_IMAGE_PREPROCESSING=true

# Performance tuning
MEMORY_LIMIT_MB=2048
CPU_PRIORITY=high
ENABLE_COMPRESSION=true
ENABLE_BACKGROUND_OCR=true

# Search optimization
SEARCH_RESULTS_PER_PAGE=50
SEARCH_SNIPPET_LENGTH=300
FUZZY_SEARCH_THRESHOLD=0.7

# Data management
RETENTION_DAYS=2555  # 7 years
ENABLE_AUTO_CLEANUP=true
```

### Runtime Settings vs Environment Variables

Some settings can be configured in two ways:

1. **Environment Variables**: Set at container startup, affects the entire application
2. **User Settings**: Configured per-user via the web interface, stored in database

**Environment variables take precedence** and provide system-wide defaults. User settings override these defaults for individual users where applicable.

Settings configurable via web interface:
- OCR language preferences
- Search result limits
- File type restrictions
- OCR processing options
- Data retention policies

### Configuration Priority

Settings are applied in this order (later values override earlier ones):

1. **Application defaults** (built into the code)
2. **Environment variables** (system-wide configuration)
3. **User settings** (per-user database settings via web interface)

This allows for flexible deployment where system administrators can set defaults while users can customize their experience.

### Quick Reference - Essential Variables

For a minimal production deployment, configure these essential variables:

```bash
# Security (REQUIRED)
JWT_SECRET=your-secure-random-key-here
DATABASE_URL=postgresql://user:password@host:port/database

# File Storage
UPLOAD_PATH=/app/uploads
WATCH_FOLDER=/path/to/mounted/folder

# Watch Folder (for network mounts)
WATCH_INTERVAL_SECONDS=60
FORCE_POLLING_WATCH=1

# Performance
CONCURRENT_OCR_JOBS=4
MAX_FILE_SIZE_MB=100
```

### Database Tuning

For better search performance with large document collections:

```sql
-- Increase shared_buffers for better caching
ALTER SYSTEM SET shared_buffers = '256MB';

-- Optimize for full-text search
ALTER SYSTEM SET default_text_search_config = 'pg_catalog.english';

-- Restart PostgreSQL after changes
```

## 🔌 API Reference

### Authentication Endpoints

```bash
# Register new user
POST /api/auth/register
Content-Type: application/json
{
  "username": "john_doe",
  "email": "john@example.com",
  "password": "secure_password"
}

# Login
POST /api/auth/login
Content-Type: application/json
{
  "username": "john_doe",
  "password": "secure_password"
}

# Get current user
GET /api/auth/me
Authorization: Bearer <jwt_token>
```

### Document Management

```bash
# Upload document
POST /api/documents
Authorization: Bearer <jwt_token>
Content-Type: multipart/form-data
file: <binary_file_data>

# List documents
GET /api/documents?limit=50&offset=0
Authorization: Bearer <jwt_token>

# Download document
GET /api/documents/{id}/download
Authorization: Bearer <jwt_token>
```

### Search

```bash
# Search documents
GET /api/search?query=contract&limit=20
Authorization: Bearer <jwt_token>

# Advanced search with filters
GET /api/search?query=invoice&mime_types=application/pdf&tags=important
Authorization: Bearer <jwt_token>
```

## 🧪 Testing

### Run All Tests

```bash
# Backend tests
cargo test

# Frontend tests
cd frontend && npm test

# Integration tests with Docker
docker compose -f docker-compose.test.yml up --build
```

### Test Coverage

```bash
# Install cargo-tarpaulin for coverage
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html
```

## 🔒 Security Considerations

### Production Deployment

1. **Change Default Credentials**: Update admin password immediately
2. **Use Strong JWT Secret**: Generate a secure random key
3. **Enable HTTPS**: Use a reverse proxy with SSL/TLS
4. **Database Security**: Use strong passwords and restrict network access
5. **File Permissions**: Ensure proper file system permissions
6. **Regular Updates**: Keep dependencies and base images updated

### Recommended Production Setup

```bash
# Use environment-specific secrets
JWT_SECRET=$(openssl rand -base64 64)

# Restrict database access
# Only allow connections from application container

# Use read-only file system where possible
# Mount uploads and watch folders as separate volumes
```

## 🚀 Deployment Options

### Docker Swarm

```yaml
version: '3.8'
services:
  readur:
    image: readur:latest
    deploy:
      replicas: 2
      restart_policy:
        condition: on-failure
    networks:
      - readur-network
    secrets:
      - jwt_secret
      - db_password
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: readur
spec:
  replicas: 3
  selector:
    matchLabels:
      app: readur
  template:
    spec:
      containers:
      - name: readur
        image: readur:latest
        env:
        - name: JWT_SECRET
          valueFrom:
            secretKeyRef:
              name: readur-secrets
              key: jwt-secret
```

### Cloud Platforms

- **AWS**: Use ECS with RDS PostgreSQL
- **Google Cloud**: Deploy to Cloud Run with Cloud SQL
- **Azure**: Use Container Instances with Azure Database
- **DigitalOcean**: App Platform with Managed Database

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup

```bash
# Fork and clone the repository
git clone https://github.com/yourusername/readur.git
cd readur

# Create a feature branch
git checkout -b feature/amazing-feature

# Make your changes and test
cargo test
cd frontend && npm test

# Submit a pull request
```

### Code Style

- **Rust**: Follow `rustfmt` and `clippy` recommendations
- **Frontend**: Use Prettier and ESLint configurations
- **Commits**: Use conventional commit format

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Tesseract OCR](https://github.com/tesseract-ocr/tesseract) for text extraction
- [Axum](https://github.com/tokio-rs/axum) for the web framework
- [Material-UI](https://mui.com/) for the beautiful frontend components
- [PostgreSQL](https://www.postgresql.org/) for robust full-text search

## 📞 Support

- **Documentation**: Check this README and inline code comments
- **Issues**: Report bugs and request features on GitHub Issues
- **Discussions**: Join community discussions on GitHub Discussions

---

**Made with ❤️ and ☕ by the Readur team**