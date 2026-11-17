# Self-Hosting Quick Start

This guide covers the essential steps for a basic self-hosted Readur deployment on a Linux server with Docker Compose.

## Prerequisites

- **A Linux server** with [Docker Engine](https://docs.docker.com/engine/install/) and the Compose plugin; verify with `docker compose version`.
- **CPU and memory for OCR.** OCR is the heaviest work Readur does. Each of the `CONCURRENT_OCR_JOBS` parallel jobs (default `4`) runs Tesseract on one document.
- **Disk space** for your documents and the PostgreSQL database.
- **Network access** to the port you publish (8000 by default), or to a reverse proxy in front of it.
- **A domain name** (optional, recommended for HTTPS).

## Step 1: Choose Your Installation Method

| Method | Best For | Updates |
|--------|----------|---------|
| **Published image** (recommended) | Most users | Pull the new image |
| **Build from source** | Development, customization | Pull from git and rebuild |

### Option A: Published Image (Recommended)

Create a directory for Readur and a `docker-compose.yml` that uses the image from the GitHub Container Registry:

```bash
mkdir readur && cd readur
```

```yaml
services:
  postgres:
    image: postgres:18
    environment:
      POSTGRES_USER: readur
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:-readur}
      POSTGRES_DB: readur
      PGDATA: /var/lib/postgresql/data
    volumes:
      - postgres_data:/var/lib/postgresql
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U readur"]
      interval: 10s
      timeout: 5s
      retries: 5
    restart: unless-stopped

  readur:
    image: ghcr.io/readur/readur:latest
    environment:
      DATABASE_URL: postgresql://readur:${POSTGRES_PASSWORD:-readur}@postgres/readur
      JWT_SECRET: ${JWT_SECRET:-}
      ADMIN_PASSWORD: ${ADMIN_PASSWORD:-}
      UPLOAD_PATH: /app/uploads
      WATCH_FOLDER: /app/watch
      OCR_LANGUAGE: eng
      CONCURRENT_OCR_JOBS: 4
    ports:
      - "8000:8000"
    volumes:
      - ./readur_uploads:/app/uploads
      - ./readur_watch:/app/watch
    depends_on:
      postgres:
        condition: service_healthy
    restart: unless-stopped

volumes:
  postgres_data:
```

`latest` is the most recent release; release versions such as `2.6.0` and `main` (the main branch) are also published.

### Option B: Build from Source

```bash
git clone https://github.com/readur/readur.git
cd readur
```

The repository's `docker-compose.yml` builds the image from the source tree and uses the same directories (`./readur_uploads`, `./readur_watch`).

## Step 2: Configure Environment

Docker Compose reads a `.env` file next to `docker-compose.yml` and substitutes its values.

**For Option A:**

```bash
cat > .env << EOF
POSTGRES_PASSWORD=$(openssl rand -hex 16)
ADMIN_PASSWORD=choose-a-long-password
EOF
```

**For Option B:** the repository's compose file takes `JWT_SECRET`, `ADMIN_USERNAME`, `ADMIN_PASSWORD` and `ADMIN_EMAIL` from `.env`; everything else, including the database password, is set directly in `docker-compose.yml`. Change the PostgreSQL password there (the `postgres` service's `POSTGRES_PASSWORD` and the password in `DATABASE_URL`) before the first start.

### Signing Key

`JWT_SECRET` is optional. Without it, Readur generates a signing key on first start and stores it in the database; `readur rotate-jwt-secret` replaces it. If you set it, use at least 32 bytes of random data:

```bash
openssl rand -hex 32
```

A shorter value or a published example is ignored with a warning, and the stored key is used.

### Admin Password

`ADMIN_PASSWORD` (at least 8 characters) is used only when the `admin` account is first created. If it is not set, Readur generates a random 24-character password on first startup and writes it to `initial-admin-password` in the `.readur` directory inside `UPLOAD_PATH` (`/app/uploads/.readur/initial-admin-password` in the container), with mode `0600`. The password is not written to the logs. `ADMIN_USERNAME` (default `admin`) and `ADMIN_EMAIL` (default `<username>@localhost`) set the account's other details; with Option A, add them to the `environment:` section to use them.

See the [Configuration Guide](../getting-started/configuration.md) for all other settings.

## Step 3: Start Services

**For Option A:**

```bash
docker compose up -d
```

**For Option B:**

```bash
docker compose up -d --build
```

The first build compiles Readur from source and takes a while.

Docker Compose starts PostgreSQL, waits for its health check, then starts Readur, which runs its database migrations automatically. Follow the startup:

```bash
docker compose logs -f readur
```

Signs of a successful start:

- `✅ SQLx migrations completed successfully`
- `✅ HTTP server bound to: 0.0.0.0:8000`

If you did not set `ADMIN_PASSWORD`, the log shows where the generated password was written:

```
🔑 The generated admin password was written to /app/uploads/.readur/initial-admin-password - sign in, change it, then delete the file
```

Read it with:

```bash
docker compose exec readur cat /app/uploads/.readur/initial-admin-password
```

After signing in and changing the password, delete the file. Press `Ctrl+C` to stop following the logs.

## Step 4: Verify Installation

```bash
docker compose ps
curl http://localhost:8000/api/health
```

Both services should be running, and the health endpoint answers `{"status":"ok"}`.

**Storage on the host:**

| Location | Purpose |
|----------|---------|
| `./readur_uploads/` | Stored documents (and `.readur/`, which holds the generated admin password file) |
| `./readur_watch/` | Files dropped here are imported automatically |
| `postgres_data` volume | The PostgreSQL database |

Docker creates the two directories when the containers start.

## Step 5: Access Readur

Open `http://localhost:8000` on the server, or `http://<server-ip>:8000` from your network, and sign in as `admin` with your `ADMIN_PASSWORD` or the generated password.

## Step 6: Initial Setup

### Upload Your First Document

Open **Intake** → **Upload** and drop PDF, image or text files onto the upload area. Each document is queued for OCR and appears in the **Library**; once OCR has finished, its text is searchable.

### Configure OCR Languages

The image includes every Tesseract language pack. Set the server default with `OCR_LANGUAGE` (for example `eng+deu`); each user can choose their own languages under **Settings** → **OCR**. Adjust `CONCURRENT_OCR_JOBS` to your server's capacity.

### Automatic Import (Optional)

**Watch folder:** copy files into `./readur_watch/` on the host:

```bash
cp ~/my-document.pdf ./readur_watch/
```

Readur imports a file once its size has stopped changing. Only extensions in `ALLOWED_FILE_TYPES` (default `pdf,txt,doc,docx,png,jpg,jpeg`) are imported. See [Watch Folder](../WATCH_FOLDER.md).

**Sources:** under **Sources** in the web interface, add a WebDAV server, an S3 bucket, or a folder on the server for Readur to sync on a schedule. See the [Sources Guide](../sources-guide.md).

## Next Steps

### Security Hardening

1. **Enable HTTPS** with a reverse proxy in front of Readur, and set `PUBLIC_URL` to its address. See [Reverse Proxy Setup](../REVERSE_PROXY.md).
2. **Configure a firewall** so that only the proxy's ports (typically 80/443) are reachable; publish Readur's port on `127.0.0.1` only.
3. **Enable SSO** (optional) with [OIDC Authentication](../oidc-setup.md).

### Performance Tuning

```bash
# Documents processed in parallel (1-15)
CONCURRENT_OCR_JOBS=4

# Largest upload, and largest PDF processed for text
MAX_FILE_SIZE_MB=50
MAX_PDF_SIZE_MB=100
```

Add these to the `environment:` section of the `readur` service. See the [Performance Tuning Guide](../performance-tuning.md).

### Backup Configuration

A simple backup script, run from the directory that holds `docker-compose.yml`:

```bash
#!/bin/bash
# backup-readur.sh
set -e
backup_dir="/path/to/backups"
stamp=$(date +%Y%m%d_%H%M%S)

# Database
docker compose exec -T postgres pg_dump -U readur readur > "$backup_dir/readur-db-$stamp.sql"

# Stored documents
tar -czf "$backup_dir/readur-uploads-$stamp.tar.gz" ./readur_uploads

echo "Backup completed: $stamp"
```

Make it executable with `chmod +x backup-readur.sh` and schedule it with cron. See the [Backup & Recovery Guide](../backup-recovery.md).

### Updating Readur

**For Option A:**

```bash
docker compose pull
docker compose up -d
```

**For Option B:**

```bash
git pull origin main
docker compose up -d --build
```

Database migrations run automatically when the new version starts. Check the [release notes](https://github.com/readur/readur/releases) before updating.

## Troubleshooting

### Container Won't Start

```bash
docker compose logs readur
docker compose logs postgres
```

Readur prints the configuration it loaded at startup, followed by the error that stopped it. Common causes are a database it cannot reach or authenticate to, and an invalid security setting (such as a malformed `TRUSTED_PROXIES` entry).

### Port Conflict

If port 8000 is already in use on the host:

```bash
ss -tlnp | grep 8000
```

Either stop the other service, or publish Readur on another host port in `docker-compose.yml`, keeping the container port `8000`:

```yaml
services:
  readur:
    ports:
      - "8080:8000"
```

Then run `docker compose up -d`.

### Database Connection Errors

```bash
docker compose ps postgres
docker compose logs postgres
```

PostgreSQL applies `POSTGRES_PASSWORD` only when it first creates the database. If you changed the password afterwards, change it back or recreate the `postgres_data` volume with `docker compose down -v` (this deletes the database).

### OCR Not Processing

```bash
docker compose logs readur | grep -i ocr
```

Common causes are documents that exceed `OCR_TIMEOUT_SECONDS` or `MAX_PDF_SIZE_MB`, too little memory for the configured `CONCURRENT_OCR_JOBS`, and unsupported file formats. The **Intake** page lists documents that need attention.

### Cannot Access from Browser

1. Check that the services are running: `docker compose ps`
2. Check that the port is listening: `ss -tlnp | grep 8000`
3. Check the firewall:
   ```bash
   # Ubuntu/Debian with UFW
   sudo ufw allow 8000/tcp

   # RHEL/CentOS with firewalld
   sudo firewall-cmd --add-port=8000/tcp --permanent
   sudo firewall-cmd --reload
   ```
4. Test locally: `curl http://localhost:8000/api/health`

### Forgot Admin Password

```bash
docker compose exec readur /app/readur reset-admin-password
```

This prints a new random password (or sets `ADMIN_PASSWORD`, when it is set in the container's environment).

## Related Documentation

- [Complete Self-Hosting Guide](../self-hosting/index.md) - Comprehensive setup and configuration
- [S3 Storage Guide](../s3-storage-guide.md) - Cloud storage configuration
- [Performance Tuning](../performance-tuning.md) - Optimize for your hardware
- [Backup & Recovery](../backup-recovery.md) - Protect your data
- [Reverse Proxy Setup](../REVERSE_PROXY.md) - HTTPS and domain configuration
