# Configuration Guide

This guide covers all configuration options available in Readur through environment variables and runtime settings.

> 📖 **See Also**: For a complete reference of all configuration options including S3 storage and advanced settings, see the [Configuration Reference](configuration-reference.md).

## Table of Contents

- [Environment Variables](#environment-variables)
  - [Core Configuration](#core-configuration)
  - [File Storage & Upload](#file-storage-upload)
  - [Watch Folder Configuration](#watch-folder-configuration)
  - [OCR & Processing Settings](#ocr-processing-settings)
  - [Search & Performance](#search-performance)
  - [Data Management](#data-management)
- [Port Configuration](#port-configuration)
- [Example Configurations](#example-configurations)
- [Configuration Priority](#configuration-priority)
- [Runtime Settings vs Environment Variables](#runtime-settings-vs-environment-variables)
- [Database Tuning](#database-tuning)

## Environment Variables

All application settings can be configured via environment variables:

### Core Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `postgresql://readur:readur@localhost/readur` | PostgreSQL connection string |
| `JWT_SECRET` | `your-secret-key` | Secret key for JWT tokens ⚠️ **Change in production!** |
| `SERVER_ADDRESS` | `0.0.0.0:8000` | Server bind address and port |

### File Storage & Upload

| Variable | Default | Description |
|----------|---------|-------------|
| `UPLOAD_PATH` | `./uploads` | Document storage directory |
| `ALLOWED_FILE_TYPES` | `pdf,txt,doc,docx,png,jpg,jpeg` | Comma-separated allowed file extensions |

### Watch Folder Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `WATCH_FOLDER` | `./watch` | Directory to monitor for new files |
| `WATCH_INTERVAL_SECONDS` | `30` | Polling interval for network filesystems (seconds) |
| `FILE_STABILITY_CHECK_MS` | `500` | Time to wait for file write completion (milliseconds) |
| `MAX_FILE_AGE_HOURS` | _(none)_ | Skip files older than this many hours |
| `FORCE_POLLING_WATCH` | _(none)_ | Force polling mode even for local filesystems |

### OCR & Processing Settings

*Note: These settings can also be configured per-user via the web interface*

| Variable | Default | Description |
|----------|---------|-------------|
| `OCR_LANGUAGE` | `eng` | OCR language code (eng, fra, deu, spa, etc.) |
| `CONCURRENT_OCR_JOBS` | `4` | Maximum parallel OCR processes |
| `OCR_TIMEOUT_SECONDS` | `300` | OCR processing timeout per file |
| `MAX_FILE_SIZE_MB` | `50` | Maximum file size for processing |
| `AUTO_ROTATE_IMAGES` | `true` | Automatically rotate images for better OCR |
| `ENABLE_IMAGE_PREPROCESSING` | `true` | Apply image enhancement before OCR |

### Search & Performance

| Variable | Default | Description |
|----------|---------|-------------|
| `SEARCH_RESULTS_PER_PAGE` | `25` | Default number of search results per page |
| `SEARCH_SNIPPET_LENGTH` | `200` | Length of text snippets in search results |
| `FUZZY_SEARCH_THRESHOLD` | `0.8` | Similarity threshold for fuzzy search (0.0-1.0) |
| `MEMORY_LIMIT_MB` | `512` | Memory limit for OCR processes |
| `CPU_PRIORITY` | `normal` | CPU priority: `low`, `normal`, `high` |

### Data Management

| Variable | Default | Description |
|----------|---------|-------------|
| `RETENTION_DAYS` | _(none)_ | Auto-delete documents after N days |
| `ENABLE_AUTO_CLEANUP` | `false` | Enable automatic cleanup of old documents |
| `ENABLE_COMPRESSION` | `false` | Compress stored documents to save space |
| `ENABLE_BACKGROUND_OCR` | `true` | Process OCR in background queue |

## Port Configuration

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

## Example Configurations

### Development Configuration

```env
# Basic development setup
DATABASE_URL=postgresql://readur:readur@localhost/readur
JWT_SECRET=dev-secret-key-not-for-production
SERVER_ADDRESS=0.0.0.0:8000
UPLOAD_PATH=./uploads
WATCH_FOLDER=./watch
OCR_LANGUAGE=eng
CONCURRENT_OCR_JOBS=2
```

### Production Configuration

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

### Network Filesystem Configuration

```env
# For NFS mounts
WATCH_FOLDER=/mnt/nfs/documents
WATCH_INTERVAL_SECONDS=60
FILE_STABILITY_CHECK_MS=1000
FORCE_POLLING_WATCH=1

# For SMB/CIFS mounts
WATCH_FOLDER=/mnt/smb/shared
WATCH_INTERVAL_SECONDS=30
FILE_STABILITY_CHECK_MS=2000

# For S3 mounts (using s3fs)
WATCH_FOLDER=/mnt/s3/bucket
WATCH_INTERVAL_SECONDS=120
FILE_STABILITY_CHECK_MS=5000
FORCE_POLLING_WATCH=1
```

## Configuration Priority

Settings are applied in this order (later values override earlier ones):

1. **Application defaults** (built into the code)
2. **Environment variables** (system-wide configuration)
3. **User settings** (per-user database settings via web interface)

This allows for flexible deployment where system administrators can set defaults while users can customize their experience.

## Runtime Settings vs Environment Variables

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

## Database Tuning

For better search performance with large document collections:

```sql
-- Increase shared_buffers for better caching
ALTER SYSTEM SET shared_buffers = '256MB';

-- Optimize for full-text search
ALTER SYSTEM SET default_text_search_config = 'pg_catalog.english';

-- Restart PostgreSQL after changes
```

## Security Configuration

### Generating Secure Secrets

```bash
# Generate secure JWT secret
JWT_SECRET=$(openssl rand -base64 64)

# Generate secure database password
DB_PASSWORD=$(openssl rand -base64 32)

# Save to .env file
cat > .env << EOF
JWT_SECRET=${JWT_SECRET}
DB_PASSWORD=${DB_PASSWORD}
EOF
```

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

## Next Steps

- Review [deployment options](deployment.md) for production setup
- Learn about [folder watching](WATCH_FOLDER.md) for automatic document ingestion
- Optimize [OCR performance](dev/OCR_OPTIMIZATION_GUIDE.md) for your use case