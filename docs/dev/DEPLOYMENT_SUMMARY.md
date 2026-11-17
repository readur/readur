# Readur Deployment Summary

A short overview of how Readur is packaged and deployed, with pointers to the full guides. For setup instructions see the [Installation Guide](../installation.md); for every setting see the [Configuration Reference](../configuration-reference.md).

## Packaging

- **Docker image**: `ghcr.io/readur/readur` (tags: `main`, `latest` for releases, and version tags such as `2.6.0`). The `Dockerfile` is a three-stage build: the React frontend is built with Node, the Rust backend with `cargo build --release`, and both are copied into a `debian:bookworm-slim` runtime image together with the database migrations. The image contains the `/app/readur` binary plus tesseract, ocrmypdf, poppler-utils, antiword, catdoc and curl, and listens on port 8000.
- **Docker Compose**: `docker-compose.yml` runs PostgreSQL and Readur. The Readur service has a health check that calls `GET /api/health` with curl. `docker-compose.dev.yml` and `docker-compose.test.yml` are for development and tests.
- **Helm**: `charts/readur` (based on the bjw-s common library chart); values in `charts/readur/values.yaml`.

## Quick start

```bash
git clone https://github.com/perfectra1n/readur
cd readur
docker compose up -d
# Open http://localhost:8000
```

On first start Readur creates the `admin` account. If `ADMIN_PASSWORD` is not set, a random password is generated and written to `<UPLOAD_PATH>/.readur/initial-admin-password`. If `JWT_SECRET` is not set, a signing key is generated and stored in the database; when set it must be at least 32 bytes.

## Configuration

All configuration is through environment variables (a `.env` file in the working directory is also loaded). There is no configuration file and there are no CLI flags for settings. Per-user preferences (OCR language, search options and so on) are stored in the database and edited on the Settings page.

Commonly changed variables:

```bash
DATABASE_URL=postgresql://readur:readur@postgres/readur
JWT_SECRET=<output of: openssl rand -hex 32>
CONCURRENT_OCR_JOBS=4
OCR_TIMEOUT_SECONDS=300
MAX_FILE_SIZE_MB=50
```

Readur has no memory or CPU-priority setting of its own; use container limits.

## Watch folder

The watch folder (`WATCH_FOLDER`, `/app/watch` in the image) is imported automatically. Readur picks a strategy per folder:

- **Polling** when the path cannot be resolved, when the resolved path starts with `//` or contains `nfs`, `smb` or `cifs`, or when `FORCE_POLLING_WATCH` is set (any value).
- **Hybrid** otherwise: filesystem notifications, with polling as a backup.

Polling uses `WATCH_INTERVAL_SECONDS` (default 30). A file is imported once its size has stayed the same for `FILE_STABILITY_CHECK_MS` (default 1000). Example for a network mount:

```yaml
services:
  readur:
    volumes:
      - /mnt/nfs/documents:/app/watch
    environment:
      FORCE_POLLING_WATCH: "1"
      WATCH_INTERVAL_SECONDS: "60"
      FILE_STABILITY_CHECK_MS: "2000"
```

See [Watch Folder](../WATCH_FOLDER.md) for details.

## Storage

Documents are stored under `UPLOAD_PATH` (`/app/uploads` in the image) or, with `S3_ENABLED=true`, in an S3 bucket. See the [S3 Storage Guide](../s3-storage-guide.md).

## Security

- Passwords are hashed with bcrypt. Login sessions use JWTs signed with `JWT_SECRET`; API keys are available for scripts and MCP clients.
- Optional OIDC login (`OIDC_*` variables, see [OIDC Setup](../oidc-setup.md)).
- Uploaded originals are kept unchanged; OCR results are stored separately.
- Upload size is limited by `MAX_FILE_SIZE_MB`; watch-folder imports are limited to `ALLOWED_FILE_TYPES`.

## Operations

- **Health**: `GET /api/health`.
- **Metrics**: `/metrics` (Prometheus text format; admin session, admin API key, or `METRICS_TOKEN` bearer token) and `/api/metrics` (JSON).
- **Logs**: stdout only, filtered with `RUST_LOG`.
- **Admin CLI**: the `reset-admin-password` and `rotate-jwt-secret` subcommands (`/app/readur reset-admin-password` inside the container); see [CLI Tools](../administration/cli-tools.md).
