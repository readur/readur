# Configuration Guide

This guide covers the most used environment variables and how to combine them. Readur reads its environment once at startup, after loading a `.env` file from the working directory (variables already set in the environment win).

> 📖 **See Also**: The [Configuration Reference](configuration-reference.md) lists every variable Readur reads, including OIDC, S3 storage and per-user watch directories.

## Table of Contents

- [Environment Variables](#environment-variables)
  - [Core Configuration](#core-configuration)
  - [File Storage & Upload](#file-storage-upload)
  - [Watch Folder Configuration](#watch-folder-configuration)
  - [OCR & Processing Settings](#ocr-processing-settings)
  - [Resources](#resources)
  - [AI Assistants (MCP)](#ai-assistants-mcp)
- [Port Configuration](#port-configuration)
- [Example Configurations](#example-configurations)
- [Configuration Priority](#configuration-priority)
- [Runtime Settings vs Environment Variables](#runtime-settings-vs-environment-variables)
- [Database Tuning](#database-tuning)

## Environment Variables

Variables not listed here or in the [Configuration Reference](configuration-reference.md) are ignored.

### Core Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `postgresql://readur:readur@localhost/readur` | PostgreSQL connection string |
| `JWT_SECRET` | _(generated, stored in the database)_ | Secret used to sign JWT tokens. Optional: when unset, a key is generated on first start and stored in the database (`readur rotate-jwt-secret` replaces it). When set to a value of at least 32 bytes that is not a published example, it takes precedence; a shorter or example value is ignored with a startup warning and the stored key is used. Generate with `openssl rand -hex 32` |
| `SERVER_ADDRESS` | `0.0.0.0:8000` | Server bind address and port (or set `SERVER_HOST` and `SERVER_PORT`) |
| `RUST_LOG` | `info` | Log level filter, e.g. `debug` or `readur=debug,sqlx=warn` |

### Security and Access Control

| Variable | Default | Description |
|----------|---------|-------------|
| `READUR_INSECURE_DEV_MODE` | `false` | Allows startup with a short or example `JWT_SECRET`. Throwaway local development and CI only; never in production |
| `JWT_TTL_HOURS` | `12` | Lifetime of login tokens in hours (1-720) |
| `ADMIN_USERNAME` | `admin` | Username of the initial admin account, which also owns files imported from `WATCH_FOLDER` |
| `ADMIN_EMAIL` | `<username>@localhost` | Email of the initial admin account |
| `ADMIN_PASSWORD` | _(none)_ | Initial admin password (min 8 chars). If unset, a random 24-character password is generated and written to a file instead of the logs |
| `ADMIN_PASSWORD_FILE` | `<UPLOAD_PATH>/.readur/initial-admin-password` | Location of the generated admin password file (mode `0600`); `/app/uploads/.readur/initial-admin-password` in the Docker image |
| `ALLOW_REGISTRATION` | `false` | Enables self-registration. New self-registered accounts are disabled until an administrator approves them in user management |
| `LOCAL_SOURCE_ALLOWED_PATHS` | _(none)_ | Comma-separated directories local folder sources may use. When set, all local folder sources (including admins') must be inside one of them. When unset, only admins can create local folder sources |
| `TRUSTED_PROXIES` | _(loopback and private networks)_ | Comma-separated IPs/CIDRs of reverse proxies whose `X-Forwarded-For` header is trusted for the client IP (rate limiting). `none` trusts no proxy |
| `CORS_ALLOWED_ORIGINS` | _(none)_ | Comma-separated origins (`scheme://host[:port]`) allowed for cross-origin requests; invalid entries stop startup. The bundled frontend is same-origin and needs none |
| `METRICS_TOKEN` | _(none)_ | Bearer token (min 16 chars) for Prometheus scrapes of `/metrics`. Without it, `/metrics` requires an admin session or API key |
| `PUBLIC_URL` | _(none)_ | Public base URL (e.g. `https://readur.example.com`) used for the OIDC redirect and shared links. OIDC falls back to the origin of `OIDC_REDIRECT_URI` |
| `OIDC_LINK_EXISTING_BY_EMAIL` | `false` | Link an OIDC login to an existing local account with the same email, only when the IdP reports `email_verified` |

See the [Configuration Reference](configuration-reference.md#security-and-access-control) for details.

### File Storage & Upload

| Variable | Default | Description |
|----------|---------|-------------|
| `UPLOAD_PATH` | `./uploads` | Document storage directory (local storage) |
| `ALLOWED_FILE_TYPES` | `pdf,txt,doc,docx,png,jpg,jpeg` | Comma-separated file extensions imported from watch folders |
| `MAX_FILE_SIZE_MB` | `50` | Maximum upload size |

To store documents in S3 or an S3-compatible service instead, see [S3 Storage](configuration-reference.md#s3-storage).

### Watch Folder Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `WATCH_FOLDER` | `./watch` | Directory to monitor for new files |
| `WATCH_INTERVAL_SECONDS` | `30` | Minimum interval between re-processing the same file; also controls polling frequency for network filesystems |
| `FILE_STABILITY_CHECK_MS` | `1000` | How long a file's size must stay unchanged before import (milliseconds) |
| `MAX_FILE_AGE_HOURS` | _(none)_ | Skip files older than this many hours |
| `FORCE_POLLING_WATCH` | _(unset)_ | Poll even on local filesystems. Any value turns it on; unset it to turn it off |
| `ENABLE_PER_USER_WATCH` | `false` | Give each user a watch directory under `USER_WATCH_BASE_DIR` (default `./user_watch`) |

### OCR & Processing Settings

Users can choose their own OCR language in Settings; `OCR_LANGUAGE` is the default.

| Variable | Default | Description |
|----------|---------|-------------|
| `OCR_LANGUAGE` | `eng` | OCR language code(s) (eng, fra, deu, spa, etc.; `eng+deu` for several) |
| `CONCURRENT_OCR_JOBS` | `4` | Documents OCRed in parallel (1-15) |
| `OCR_TIMEOUT_SECONDS` | `300` | OCR processing timeout per file |
| `MAX_PDF_SIZE_MB` | `100` | Maximum PDF file size for OCR processing |
| `MAX_OFFICE_DOCUMENT_SIZE_MB` | `100` | Maximum Office document size for text extraction |
| `OCR_LOW_MEMORY_MODE` | `false` | Read only by the image OCR service behind `GET /api/ocr/health`; document processing does not use it |

The fuzzy search threshold is not an environment variable: each user sets it under Settings (Search configuration), default 0.5. See [Fuzzy search](advanced-search.md#fuzzy-search-similar-spelling).

### Resources

| Variable | Default | Description |
|----------|---------|-------------|
| `MEMORY_LIMIT_MB` | `512` | Shown in the startup log only; does not limit memory. Limit the container instead |
| `CPU_PRIORITY` | `normal` | Shown in the startup log only; does not change process priority |

### AI Assistants (MCP)

| Variable | Default | Description |
|----------|---------|-------------|
| `MCP_ENABLED` | `true` | Serve the Model Context Protocol endpoint at `/api/mcp` (requires an API key or session token) |
| `MCP_ALLOW_DELETE` | `false` | Offer the `delete_document` tool to MCP clients |

See [AI Assistants (MCP)](mcp-guide.md) for connecting Claude and other clients.

## Port Configuration

Readur supports flexible port configuration:

```bash
# Method 1: Specify full server address
SERVER_ADDRESS=0.0.0.0:8000

# Method 2: Use separate host and port (recommended)
SERVER_HOST=0.0.0.0
SERVER_PORT=8000

# Frontend development server only (read by frontend/vite.config.ts)
CLIENT_PORT=5173
BACKEND_PORT=8000
```

## Example Configurations

### Development Configuration

```env
# Basic development setup
DATABASE_URL=postgresql://readur:readur@localhost/readur
JWT_SECRET=readur-local-development-only-jwt-secret-0123456789
READUR_INSECURE_DEV_MODE=true   # the example secret above is only accepted in dev mode
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
JWT_SECRET=<output of: openssl rand -hex 32>
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
MAX_PDF_SIZE_MB=200
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
JWT_SECRET=$(openssl rand -hex 32)

# Generate secure database password
POSTGRES_PASSWORD=$(openssl rand -hex 24)

# Save to .env file
cat > .env << EOF
JWT_SECRET=${JWT_SECRET}
POSTGRES_PASSWORD=${POSTGRES_PASSWORD}
EOF
```

### Quick Reference - Essential Variables

For a minimal production deployment, configure these essential variables:

```bash
# Security (REQUIRED)
JWT_SECRET=<output of: openssl rand -hex 32>   # optional; min 32 bytes when set
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