# Self-Hosting Guide

This guide covers running Readur on your own infrastructure: choosing a deployment method, the settings you need for production, and the day-to-day work of keeping an installation healthy.

## What You Run

A Readur installation has two parts:

- **Readur**, a single server process (`/app/readur` in the image `ghcr.io/readur/readur`) that serves the web interface and the REST API on port 8000, runs OCR, imports from watch folders and sources, and runs database migrations automatically at startup.
- **PostgreSQL**, which holds users, settings, document metadata and OCR text.

Document files are stored on the local filesystem (`UPLOAD_PATH`) or in an S3 bucket. There is no cache server, message queue or separate worker to run. Readur is a single-instance application: run one server per database and give it more CPU and memory to scale.

## Choosing Your Deployment Method

### Docker Compose (Recommended)

Docker Compose runs Readur and PostgreSQL together on one host and is the simplest way to get started.

[Docker Quick Start →](../quickstart/docker.md)

### Kubernetes with Helm

The chart in `charts/readur` (built on the bjw-s common library) deploys Readur into an existing cluster. It does not deploy PostgreSQL, and it runs a single replica.

[Installation Guide: Kubernetes with Helm →](../getting-started/installation.md#kubernetes-with-helm)

### Running the Binary Directly

Readur can also be built from source and run as a regular service; it then needs Tesseract and the other OCR tools installed on the host.

[Manual installation →](../installation.md#manual-installation)

## System Requirements

For a small installation, plan on at least 2 CPU cores, 2 GB of RAM and 10 GB of free disk space, plus room for your documents. For production, 4 or more cores, 4 GB or more of RAM and SSD storage give OCR room to work. OCR is the heaviest load: more cores let you raise `CONCURRENT_OCR_JOBS`, and large scanned PDFs need more memory. See [Performance Tuning](./performance.md).

## Installation with Docker Compose

```bash
git clone https://github.com/readur/readur.git
cd readur

# Optional: choose the initial admin password (otherwise one is generated)
echo "ADMIN_PASSWORD=choose-a-long-password" > .env

docker compose up -d --build
docker compose ps
curl http://localhost:8000/api/health
```

The repository's `docker-compose.yml` builds the image from source; to use the published image instead, replace `build: .` with `image: ghcr.io/readur/readur:latest`. Startup is complete when the log (`docker compose logs -f readur`) shows `✅ HTTP server bound to: 0.0.0.0:8000`, and the health endpoint answers `{"status":"ok"}`.

If you did not set `ADMIN_PASSWORD`, read the generated one:

```bash
docker compose exec readur cat /app/uploads/.readur/initial-admin-password
```

[Detailed installation guide →](../getting-started/installation.md)

## Configuration

Readur is configured only with environment variables, read at startup (a `.env` file in the working directory is also loaded). The [Configuration Reference](../configuration-reference.md) lists every variable; anything else is ignored.

### Settings to Review Before Going Live

```bash
# Database
DATABASE_URL=postgresql://readur:a-strong-password@postgres/readur

# Initial administrator (only used when the account does not exist yet)
ADMIN_USERNAME=admin
ADMIN_PASSWORD=choose-a-long-password

# Public address, used for OIDC redirects and shared links; enables HSTS when https
PUBLIC_URL=https://readur.example.com

# Optional: session signing key (at least 32 bytes). When unset, one is generated
# and stored in the database.
JWT_SECRET=
```

Change the PostgreSQL password in both the `postgres` service and `DATABASE_URL` before the database is first created. `JWT_SECRET` is optional; if you set it, generate it with `openssl rand -hex 32` and keep it stable, since changing it signs everyone out.

### Storage

Documents are stored under `UPLOAD_PATH` (`/app/uploads` in the image) unless S3 storage is enabled:

```bash
S3_ENABLED=true
S3_BUCKET_NAME=readur-documents
S3_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
S3_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
# For MinIO and other S3-compatible services:
S3_ENDPOINT_URL=http://minio:9000
```

If S3 is enabled but misconfigured, Readur logs the problem and falls back to local storage, so check the startup log.

[Storage Configuration →](./storage.md)

### Authentication

Local username/password accounts are on by default; self-registration is off (`ALLOW_REGISTRATION=false`), and self-registered accounts must be approved by an administrator. For single sign-on, connect an OIDC provider:

```bash
OIDC_ENABLED=true
OIDC_CLIENT_ID=readur
OIDC_CLIENT_SECRET=your-client-secret
OIDC_ISSUER_URL=https://sso.example.com/realms/main
OIDC_REDIRECT_URI=https://readur.example.com/api/auth/oidc/callback
OIDC_AUTO_REGISTER=true
```

Scripts and AI assistants use personal API keys, created in **Settings → API keys**.

[Authentication Configuration →](./authentication.md)

## Network Configuration

### HTTPS with a Reverse Proxy

Readur speaks plain HTTP, so put a reverse proxy in front of it for HTTPS:

```nginx
server {
    listen 443 ssl http2;
    server_name readur.example.com;

    ssl_certificate     /etc/letsencrypt/live/readur.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/readur.example.com/privkey.pem;

    client_max_body_size 100M;

    location / {
        proxy_pass http://127.0.0.1:8000;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

Set `PUBLIC_URL` to the public address, and `TRUSTED_PROXIES` if the proxy is not on the same host or a private network.

[Reverse Proxy Configuration →](./reverse-proxy.md)

### Firewall

Only the proxy needs to be reachable from outside. Keep port 8000 and PostgreSQL's port 5432 private (the repository's `docker-compose.yml` publishes both on the host; remove those `ports:` entries when a proxy on the same Docker network fronts Readur):

```bash
sudo ufw allow 443/tcp
sudo ufw allow 80/tcp
sudo ufw allow from 192.0.2.10 to any port 22
sudo ufw enable
```

Note that Docker's published ports bypass `ufw` rules, which is another reason not to publish ports you do not need.

## Backup and Recovery

Back up both the database and the document files (the `UPLOAD_PATH` directory, or your S3 bucket):

```bash
backup_date=$(date +%Y%m%d)
docker compose exec -T postgres pg_dump -U readur readur > readur-db-$backup_date.sql
tar -czf readur-uploads-$backup_date.tar.gz ./readur_uploads
```

To restore, stop Readur, recreate the database and load the dump into it, restore the files, and start Readur again:

```bash
docker compose stop readur
docker compose exec -T postgres dropdb -U readur --if-exists readur
docker compose exec -T postgres createdb -U readur -O readur readur
docker compose exec -T postgres psql -U readur -d readur < readur-db-20260301.sql
tar -xzf readur-uploads-20260301.tar.gz
docker compose start readur
```

[Backup Strategies →](./backup.md)

## Monitoring and Maintenance

### Health and Metrics

```bash
# Liveness check, no authentication: {"status":"ok"}
curl http://localhost:8000/api/health

# Prometheus metrics (admin session or API key, or METRICS_TOKEN)
curl -H "Authorization: Bearer $metrics_token" http://localhost:8000/metrics

# The same figures as JSON (admin)
curl -H "Authorization: Bearer $token" http://localhost:8000/api/metrics
```

Logs go to standard output (`docker compose logs readur`); set `RUST_LOG=debug` for more detail.

[Monitoring →](./monitoring.md)

### Performance Tuning

The main knob is `CONCURRENT_OCR_JOBS` (documents OCRed in parallel, default 4, clamped to 1-15), with `OCR_TIMEOUT_SECONDS` and `MAX_FILE_SIZE_MB` alongside it.

[Performance Tuning →](./performance.md)

### Updates

Back up first, then pull the new image and recreate the container. Database migrations run automatically when the new version starts:

```bash
docker compose pull
docker compose up -d
docker compose logs -f readur
```

When building from source, run `git pull` and `docker compose up -d --build` instead.

[Upgrading Readur →](../migration-guide.md#upgrading-readur)

## Troubleshooting

**The server does not start:** read the startup log (`docker compose logs readur`). Readur prints the configuration it loaded and stops with an error on invalid security settings or when it cannot reach the database.

**Database connection failed:** check `DATABASE_URL` and that PostgreSQL is running:

```bash
docker compose exec postgres pg_isready -U readur
```

**OCR is not processing:** check the queue as an administrator (`GET /api/queue/stats`) and whether it is paused (`GET /api/queue/status`); OCR runs inside the Readur server, so its errors appear in `docker compose logs readur`.

**Lost the admin password:** run `docker compose exec readur ./readur reset-admin-password` (see [CLI Tools](../administration/cli-tools.md)).

[Troubleshooting Guide →](../troubleshooting.md)

## Migrating from Other Systems

Readur has no importer for other document systems' databases. Export the original files and bring them in through a watch folder, a source or the upload API; see [Bringing Existing Documents into Readur](../migration-guide.md#bringing-existing-documents-into-readur).

## Getting Help

- **GitHub Issues**: [Report bugs](https://github.com/readur/readur/issues)
- **Discussions**: [Ask questions](https://github.com/readur/readur/discussions)

## Next Steps

1. [Install Readur](../getting-started/installation.md)
2. [Configure storage](./storage.md)
3. [Set up authentication](./authentication.md)
4. [Enable HTTPS](./reverse-proxy.md)
5. [Configure backups](./backup.md)
6. [Set up monitoring](./monitoring.md)
