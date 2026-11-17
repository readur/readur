# Configuration Guide

Readur is configured with environment variables. This page explains how configuration works and walks through the settings most installations need. The [Configuration Reference](../configuration-reference.md) is the complete list of every variable Readur reads, with types and defaults; anything not listed there is ignored.

## How Configuration Works

- Readur reads its environment once at startup. After changing a variable, restart the server.
- A `.env` file in the working directory is loaded first. A variable that is already set in the environment (for example by Docker Compose or Kubernetes) takes precedence over the `.env` file.
- There is no configuration file and there are no command-line options for settings. The `readur` binary has only the subcommands `serve` (the default), `reset-admin-password` and `rotate-jwt-secret`.
- Per-user preferences, such as OCR languages and search options, are not environment variables. Each user sets them on the **Settings** page of the web interface, and they are stored in the database.

### Where to Set Variables

**Docker Compose:** under `environment:` of the `readur` service, or in a `.env` file next to `docker-compose.yml` that the compose file references with `${VARIABLE}`. The `docker-compose.yml` in the repository passes `JWT_SECRET`, `ADMIN_USERNAME`, `ADMIN_PASSWORD` and `ADMIN_EMAIL` through from the shell or `.env` file and sets the rest directly:

```yaml
services:
  readur:
    environment:
      DATABASE_URL: postgresql://readur:readur@postgres/readur
      UPLOAD_PATH: /app/uploads
      WATCH_FOLDER: /app/watch
      OCR_LANGUAGE: eng
      ADMIN_PASSWORD: ${ADMIN_PASSWORD:-}
```

**Helm:** under `controllers.main.containers.readur.env` in your values file. The chart manages `JWT_SECRET` and `ADMIN_PASSWORD` itself through a Secret; see [Helm Chart Values](../configuration-reference.md#helm-chart-values).

**Running the binary directly:** export the variables in the shell, or put them in a `.env` file in the directory you start `readur` from.

## Essential Settings

### Database

Readur needs a PostgreSQL database. Set either a connection string:

```bash
DATABASE_URL=postgresql://readur:your_password@db.example.com:5432/readur
```

or the individual parts, which are used only when `DATABASE_URL` is not set:

```bash
POSTGRES_HOST=db.example.com
POSTGRES_PORT=5432
POSTGRES_DB=readur
POSTGRES_USER=readur
POSTGRES_PASSWORD=your_password
```

Database migrations run automatically at startup.

### Initial Admin Account

On first startup Readur creates an administrator account. `ADMIN_USERNAME` (default `admin`), `ADMIN_EMAIL` and `ADMIN_PASSWORD` are used only at that moment; changing them later does not change the existing account.

If `ADMIN_PASSWORD` is not set, Readur generates a random 24-character password and writes it to `.readur/initial-admin-password` inside `UPLOAD_PATH` (`/app/uploads/.readur/initial-admin-password` in the Docker image), with mode `0600`. The password is not written to the logs; the startup log shows the file path. Sign in, change the password, then delete the file. `ADMIN_PASSWORD_FILE` chooses a different location.

To set a new admin password later, run `readur reset-admin-password` (see [CLI Tools](../administration/cli-tools.md)).

### Session Signing Key

`JWT_SECRET` is optional. When it is unset, Readur generates a signing key on first start and stores it in the database; `readur rotate-jwt-secret` replaces it. To manage the key yourself, set `JWT_SECRET` to at least 32 bytes of random data:

```bash
openssl rand -hex 32
```

A value that is shorter than 32 bytes or a published example (such as `your-secret-key` or `change-me`) is ignored with a warning in the startup log, and the stored key is used instead. `JWT_TTL_HOURS` (default `12`, range 1-720) sets how long a login lasts.

### Storage Paths

| Variable | Default | Docker image |
|----------|---------|--------------|
| `UPLOAD_PATH` | `./uploads` | `/app/uploads` |
| `WATCH_FOLDER` | `./watch` | `/app/watch` |

Mount persistent volumes at these paths in containers. `UPLOAD_PATH` holds every stored document when the local storage backend is used.

### Listening Address

Readur listens on `0.0.0.0:8000` by default. Change it with `SERVER_ADDRESS` (for example `0.0.0.0:9000`), or with `SERVER_HOST` and `SERVER_PORT`.

## Common Tasks

### Running Behind a Reverse Proxy

```bash
PUBLIC_URL=https://readur.example.com
TRUSTED_PROXIES=10.0.0.0/8
```

`PUBLIC_URL` is used to build OIDC redirects and shared-link URLs. `TRUSTED_PROXIES` lists the proxies whose `X-Forwarded-For` header is trusted when determining the client IP; by default loopback and private networks are trusted. See the [Reverse Proxy Guide](../REVERSE_PROXY.md).

### OCR and File Limits

```bash
OCR_LANGUAGE=eng+deu
CONCURRENT_OCR_JOBS=4
OCR_TIMEOUT_SECONDS=300
MAX_FILE_SIZE_MB=50
```

- `OCR_LANGUAGE` is the default Tesseract language code, or several joined with `+`. Users can choose their own languages in **Settings**. The Docker image includes all Tesseract language packs.
- `CONCURRENT_OCR_JOBS` is the number of documents processed in parallel (clamped to 1-15).
- `OCR_TIMEOUT_SECONDS` is the time limit for processing one document.
- `MAX_FILE_SIZE_MB` is the largest accepted upload. `MAX_PDF_SIZE_MB` and `MAX_OFFICE_DOCUMENT_SIZE_MB` (both default `100`) limit which PDFs and Office documents are processed for text.

Readur has no memory or CPU-priority setting of its own. To cap memory or CPU, use container limits.

### Watch Folder

```bash
WATCH_FOLDER=/app/watch
WATCH_INTERVAL_SECONDS=30
FILE_STABILITY_CHECK_MS=1000
ALLOWED_FILE_TYPES=pdf,txt,doc,docx,png,jpg,jpeg
```

New files in `WATCH_FOLDER` are imported once their size has stopped changing for `FILE_STABILITY_CHECK_MS`. Only extensions in `ALLOWED_FILE_TYPES` are imported. `MAX_FILE_AGE_HOURS` skips files last modified longer ago than that (the repository's `docker-compose.yml` sets it to `24`). Network filesystems are polled; `FORCE_POLLING_WATCH` forces polling everywhere. For a watch directory per user, set `ENABLE_PER_USER_WATCH=true` and `USER_WATCH_BASE_DIR`. See [Watch Folder](../WATCH_FOLDER.md) and [Per-User Watch Directories](../per-user-watch-directories.md).

Folders on the server can also be added as local folder sources in the web interface. Unless `LOCAL_SOURCE_ALLOWED_PATHS` is set, only administrators can create them.

### S3 Storage

To store documents in S3 or an S3-compatible service instead of `UPLOAD_PATH`:

```bash
S3_ENABLED=true
S3_BUCKET_NAME=readur-documents
S3_REGION=us-east-1
S3_ACCESS_KEY_ID=your_access_key
S3_SECRET_ACCESS_KEY=your_secret_key
# Only for S3-compatible services such as MinIO:
S3_ENDPOINT_URL=https://s3.example.com
```

With a custom endpoint, path-style addressing is used unless `S3_FORCE_PATH_STYLE` says otherwise. See [S3 Storage](../configuration-reference.md#s3-storage) and the [S3 Storage Guide](../s3-storage-guide.md).

### Single Sign-On (OIDC)

```bash
OIDC_ENABLED=true
OIDC_ISSUER_URL=https://auth.example.com/realms/main
OIDC_CLIENT_ID=readur
OIDC_CLIENT_SECRET=your-client-secret
OIDC_REDIRECT_URI=https://readur.example.com/api/auth/oidc/callback
OIDC_AUTO_REGISTER=true
```

`ALLOW_LOCAL_AUTH=false` turns off username/password login, and `OIDC_LINK_EXISTING_BY_EMAIL=true` links an OIDC login to an existing account with the same verified email. See the [OIDC Setup Guide](../oidc-setup.md).

### User Registration

Self-registration is off by default. With `ALLOW_REGISTRATION=true`, people can create accounts from the login page, but each new account stays disabled until an administrator enables it in user management.

### Monitoring

`GET /api/health` answers `{"status":"ok"}` without authentication. Prometheus metrics at `/metrics` require an admin session or admin API key; for a scraper, set a bearer token of at least 16 characters:

```bash
METRICS_TOKEN=<output of: openssl rand -hex 24>
```

See the [Health Monitoring Guide](../health-monitoring-guide.md).

### AI Assistants (MCP)

The Model Context Protocol endpoint at `/api/mcp` is on by default (`MCP_ENABLED`) and needs an API key or session token. The `delete_document` tool is offered only with `MCP_ALLOW_DELETE=true`. See [AI Assistants (MCP)](../mcp-guide.md).

### Logging

Logs go to standard output. `RUST_LOG` filters them, using [tracing `EnvFilter`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html) syntax:

```bash
RUST_LOG=info
RUST_LOG=readur=debug,sqlx=warn
```

`DEBUG=true` adds extra diagnostic log lines.

## Example: Small Docker Compose Installation

```bash
# .env next to docker-compose.yml
ADMIN_PASSWORD=choose-a-long-password
```

```yaml
services:
  readur:
    image: ghcr.io/readur/readur:latest
    environment:
      DATABASE_URL: postgresql://readur:readur@postgres/readur
      ADMIN_PASSWORD: ${ADMIN_PASSWORD:-}
      UPLOAD_PATH: /app/uploads
      WATCH_FOLDER: /app/watch
      OCR_LANGUAGE: eng
      CONCURRENT_OCR_JOBS: 2
      PUBLIC_URL: https://readur.example.com
```

## Checking the Configuration

At startup Readur prints the settings it loaded, marking which came from the environment and which are defaults:

```bash
docker compose logs readur
```

Invalid security settings (for example a malformed `TRUSTED_PROXIES` entry or an invalid `CORS_ALLOWED_ORIGINS` origin) stop the server with an error; an invalid number falls back to its default with a warning. Administrators can read the effective server configuration, without secrets, from `GET /api/settings/config` or the **Settings** page.

If a variable seems to have no effect, check its name against the [Configuration Reference](../configuration-reference.md), make sure there are no spaces around `=`, and restart the server.

## Next Steps

- [Configuration Reference](../configuration-reference.md) - every variable, with defaults
- [Installation Guide](installation.md) - deploy Readur
- [User Guide](../user-guide.md) - learn the interface
- [Deployment Guide](../deployment.md) - production setup
