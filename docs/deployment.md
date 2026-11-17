# Deployment Guide

This guide covers running Readur in production: the container image, Docker Compose, volume paths on common platforms, network-mounted watch folders, a reverse proxy, health checks, backups, monitoring, Kubernetes with the Helm chart, and a security checklist.

## Table of Contents

- [Container Image](#container-image)
- [Production Docker Compose](#production-docker-compose)
- [Volume Path Syntax](#volume-path-syntax)
- [Platform-Specific Deployment Notes](#platform-specific-deployment-notes)
  - [Synology NAS](synology.md) - Complete Synology deployment guide
- [Network Filesystem Mounts](#network-filesystem-mounts)
- [SSL/HTTPS Setup](#sslhttps-setup)
  - [Nginx Configuration](#nginx-configuration)
  - [Traefik Configuration](#traefik-configuration)
- [Health Checks](#health-checks)
- [Backup Strategy](#backup-strategy)
- [Upgrading PostgreSQL](postgres-upgrade.md)
- [Monitoring](#monitoring)
- [Kubernetes](#kubernetes)
- [Security Considerations](#security-considerations)

## Container Image

Readur is published as `ghcr.io/readur/readur` for `linux/amd64` and `linux/arm64`. Tags:

| Tag | Contents |
|-----|----------|
| `latest` | The most recent release |
| `3.1.3`, `3.1`, `3` | A release, or the newest release of a minor or major version |
| `main` | The current development branch; may change at any time |

Pin a release tag in production so that an update happens only when you change it.

The image runs `/app/readur` (the server) from `/app`, listens on port 8000, and contains Tesseract with all language packs, `ocrmypdf`, `poppler-utils`, `antiword`, `catdoc` and `curl`. It runs as root unless you set a user. Data lives in two directories:

| Path | Setting | Contents |
|------|---------|----------|
| `/app/uploads` | `UPLOAD_PATH` | Stored documents (with local storage), thumbnails, and `.readur/initial-admin-password` |
| `/app/watch` | `WATCH_FOLDER` | Watch folder; files placed here are imported, and never modified or deleted |

PostgreSQL is not included; run it as a separate container or service. Readur creates its tables on startup (the database user must be allowed to create the `pg_trgm` extension, which the database owner can on PostgreSQL 13 and later).

## Production Docker Compose

```yaml
services:
  readur:
    image: ghcr.io/readur/readur:3.1.3
    ports:
      - "8000:8000"
    environment:
      DATABASE_URL: postgresql://readur:${DB_PASSWORD}@postgres:5432/readur
      UPLOAD_PATH: /app/uploads
      WATCH_FOLDER: /app/watch
      PUBLIC_URL: https://readur.example.com
      OCR_LANGUAGE: eng
      CONCURRENT_OCR_JOBS: "4"
      OCR_TIMEOUT_SECONDS: "300"
      MAX_FILE_SIZE_MB: "50"
    volumes:
      - ./data/uploads:/app/uploads
      - ./data/watch:/app/watch
    depends_on:
      postgres:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/api/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    restart: unless-stopped
    deploy:
      resources:
        limits:
          cpus: "2.0"
          memory: 2G

  postgres:
    image: postgres:18
    environment:
      POSTGRES_USER: readur
      POSTGRES_PASSWORD: ${DB_PASSWORD}
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
    # No published port: only Readur, on the Compose network, needs the database.

volumes:
  postgres_data:
```

Create a `.env` file next to it with the database password, then start the stack:

```bash
printf 'DB_PASSWORD=%s\n' "$(openssl rand -hex 24)" > .env
chmod 600 .env
docker compose up -d
```

Notes:

- `JWT_SECRET` is optional. When it is unset, Readur generates a signing key on first start and stores it in the database. If you set it, use a random value of at least 32 bytes and keep it the same across restarts (changing it signs everyone out); a shorter or published example value is ignored with a warning in the startup log, and the stored key is used instead.
- When `ADMIN_PASSWORD` is not set, the initial `admin` password is written to `./data/uploads/.readur/initial-admin-password` on the host. Sign in, change it, and delete the file.
- Self-registration is off (`ALLOW_REGISTRATION=false`); create accounts in user management.
- Keep the PostgreSQL major version you started with. Moving to a new major version needs a dump and restore; see the [PostgreSQL Upgrade Guide](postgres-upgrade.md). If you use an auto-update tool (Watchtower, What's Up Docker), exclude the `postgres` container from it.
- Run one Readur instance per database. Each server process starts its own OCR worker, watch-folder watcher and source sync scheduler, and nothing coordinates these between instances.
- Every setting is listed in the [Configuration Reference](configuration-reference.md); see [Performance Tuning](performance-tuning.md) for the OCR settings.

## Volume Path Syntax

Docker volume paths come in three forms:

| Syntax | Example | Meaning |
|--------|---------|---------|
| `./path` | `./data/uploads:/app/uploads` | Relative to the directory of the `docker-compose.yml` file |
| `/path` | `/srv/readur/uploads:/app/uploads` | Absolute path on the host |
| `name` | `postgres_data:/var/lib/postgresql` | Named volume managed by Docker |

Common mistakes:

- **A relative path where an absolute one was meant.** `./DATA/AppData/readur/uploads` is created next to the compose file; `/DATA/AppData/readur/uploads` is the host directory.
- **Case.** Linux paths are case-sensitive: `/DATA` and `/data` are different directories.
- **A missing or read-only parent directory.** Docker creates a missing bind-mount directory only if it can write to the parent. Create the directories first:

```bash
mkdir -p /srv/readur/uploads /srv/readur/watch
```

A good default is a named volume for the database and bind mounts for the directories you want to reach from the host (uploads and the watch folder).

## Platform-Specific Deployment Notes

The examples show the `volumes` entries only; combine them with the compose file above.

### Portainer

In Portainer stacks, relative paths (`./...`) resolve against Portainer's own working directory. Use absolute paths and create the directories beforehand:

```yaml
services:
  readur:
    volumes:
      - /srv/readur/uploads:/app/uploads
      - /srv/readur/watch:/app/watch
```

### ZimaOS

ZimaOS keeps application data under `/DATA/AppData/` (absolute, upper case):

```yaml
services:
  readur:
    volumes:
      - /DATA/AppData/readur/uploads:/app/uploads
      - /DATA/AppData/readur/watch:/app/watch
```

### Unraid

```yaml
services:
  readur:
    volumes:
      - /mnt/user/appdata/readur/uploads:/app/uploads
      - /mnt/user/appdata/readur/watch:/app/watch
```

### Synology DSM

Use a named volume for the PostgreSQL data, do not publish the PostgreSQL port (or publish it on another host port such as 5433), and create the shared folders in File Station first. See the **[Synology Deployment Guide](synology.md)**.

### QNAP

```yaml
services:
  readur:
    volumes:
      - /share/Container/readur/uploads:/app/uploads
      - /share/Container/readur/watch:/app/watch
```

### TrueNAS SCALE

Use dataset paths:

```yaml
services:
  readur:
    volumes:
      - /mnt/pool/apps/readur/uploads:/app/uploads
      - /mnt/pool/apps/readur/watch:/app/watch
```

## Network Filesystem Mounts

The watch folder can be a network share mounted on the host and bind-mounted into the container. Readur does not receive filesystem events from NFS and SMB/CIFS mounts, so it polls paths that look like such mounts every `WATCH_INTERVAL_SECONDS`. If a network mount is not detected (for example a FUSE mount), set `FORCE_POLLING_WATCH=1`. Raise `FILE_STABILITY_CHECK_MS` if files arrive slowly, so that a file is imported only after its size has stopped changing.

```yaml
services:
  readur:
    volumes:
      - /mnt/nfs/documents:/app/watch:ro
    environment:
      WATCH_INTERVAL_SECONDS: "60"
      FILE_STABILITY_CHECK_MS: "2000"
```

Readur only reads the watch folder, so it can be mounted read-only (`:ro`). See [Watch Folder](WATCH_FOLDER.md) for details.

For WebDAV servers (Nextcloud, ownCloud), S3 buckets or server folders, a [source](sources-guide.md) is usually a better fit than a mounted watch folder. To store documents in S3 instead of `/app/uploads`, see the [S3 Storage Guide](s3-storage-guide.md).

## SSL/HTTPS Setup

Readur serves plain HTTP; terminate TLS in a reverse proxy. Behind a proxy:

- Set `PUBLIC_URL` to the public address (for example `https://readur.example.com`). Readur uses it for the OIDC redirect after sign-in and for shared-link URLs; it does not derive them from request headers.
- `X-Forwarded-For` is trusted only from `TRUSTED_PROXIES` (by default loopback and private networks, which covers a proxy on the same host or Docker network).
- Allow request bodies at least as large as `MAX_FILE_SIZE_MB`, or uploads fail in the proxy before reaching Readur.
- Pass WebSocket upgrades: the live sync progress of a source uses `GET /api/sources/{id}/sync/progress/ws`.

### Nginx Configuration

```nginx
map $http_upgrade $connection_upgrade {
    default upgrade;
    ''      close;
}

server {
    listen 443 ssl http2;
    server_name readur.example.com;

    ssl_certificate     /etc/nginx/ssl/readur.example.com.crt;
    ssl_certificate_key /etc/nginx/ssl/readur.example.com.key;

    client_max_body_size 50M;   # at least MAX_FILE_SIZE_MB

    location / {
        proxy_pass http://127.0.0.1:8000;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $connection_upgrade;
        proxy_read_timeout 300s;
        proxy_send_timeout 300s;
    }
}
```

Keep the `Host` header: the WebSocket endpoint accepts a browser's `Origin` when it matches the `Host` header, `PUBLIC_URL`, an entry in `CORS_ALLOWED_ORIGINS`, or (for a trusted proxy) `X-Forwarded-Host`.

### Traefik Configuration

```yaml
services:
  readur:
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.readur.rule=Host(`readur.example.com`)"
      - "traefik.http.routers.readur.entrypoints=websecure"
      - "traefik.http.routers.readur.tls.certresolver=letsencrypt"
      - "traefik.http.services.readur.loadbalancer.server.port=8000"
```

Traefik passes WebSocket upgrades and has no request size limit by default. For Apache, Caddy and custom ports, see [Reverse Proxy](REVERSE_PROXY.md).

## Health Checks

`GET /api/health` needs no authentication and returns `200 OK` with `{"status": "ok"}` while the server is up. The image includes `curl`:

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

The port in the URL is the port Readur listens on inside the container: `SERVER_PORT` (default 8000) or the port in `SERVER_ADDRESS`. If you set `SERVER_PORT=3000`, check `http://localhost:3000/api/health`.

## Backup Strategy

> **Upgrading PostgreSQL?** See the [PostgreSQL Upgrade Guide](postgres-upgrade.md).

A complete backup has two parts, taken together:

1. **The database**: users, settings, document records, extracted text, labels, sources, API keys and the generated JWT signing key.
2. **The stored files**: the upload directory (`/app/uploads`), or the S3 bucket when S3 storage is used.

The watch folder holds your originals, which you back up as you would any other files.

Example script for the compose file above, run from its directory:

```bash
#!/bin/bash
# backup.sh - database dump and upload archive
set -euo pipefail

backup_dir=/srv/backups/readur
stamp=$(date +%Y%m%d_%H%M%S)
mkdir -p "$backup_dir"

docker compose exec -T postgres pg_dump -U readur -Fc readur > "$backup_dir/readur_$stamp.dump"
tar -czf "$backup_dir/uploads_$stamp.tar.gz" -C ./data uploads

# Keep 30 days
find "$backup_dir" -name 'readur_*.dump' -mtime +30 -delete
find "$backup_dir" -name 'uploads_*.tar.gz' -mtime +30 -delete
```

Schedule it with cron, for example daily at 02:00:

```bash
0 2 * * * cd /srv/readur && ./backup.sh >> /var/log/readur-backup.log 2>&1
```

To keep the dump and the files consistent with each other, you can pause OCR first (`POST /api/queue/pause`, admin) and resume it afterwards (`POST /api/queue/resume`), or stop the `readur` container during the backup.

### Restore from Backup

Restore into an empty database, with Readur stopped:

```bash
docker compose stop readur
docker compose exec -T postgres pg_restore -U readur -d readur --clean --if-exists < /srv/backups/readur/readur_20260101_020000.dump
tar -xzf /srv/backups/readur/uploads_20260101_020000.tar.gz -C ./data
docker compose start readur
```

## Monitoring

```bash
# Resource usage
docker stats

# Server log (Readur logs to stdout; RUST_LOG sets the level)
docker compose logs -f readur
```

The admin endpoints `GET /api/queue/stats` (OCR queue counts) and `GET /api/metrics` (JSON) show the state of processing. See the [Health Monitoring Guide](health-monitoring-guide.md) for more.

### Prometheus Metrics

`/metrics` serves the same figures in Prometheus text format. It needs an admin session or admin API key, or a bearer token: set `METRICS_TOKEN` (at least 16 characters) on the server and give Prometheus the same value.

```yaml
# prometheus.yml
scrape_configs:
  - job_name: readur
    authorization:
      type: Bearer
      credentials_file: /etc/prometheus/readur-metrics-token
    static_configs:
      - targets: ["readur:8000"]
```

## Kubernetes

The Helm chart is in `charts/readur` in the repository. It is built on the [bjw-s common library chart](https://github.com/bjw-s-labs/helm-charts), so its values follow that chart's layout (`controllers`, `persistence`, `service`, `ingress`). Chart releases are also pushed as OCI artifacts to `oci://ghcr.io/readur/readur/readur`.

What the chart creates:

- One Deployment with **one replica** and the `Recreate` strategy, running as UID/GID 1000 with an init container that sets the ownership of `/app/uploads` and `/app/watch` to 1000:1000.
- Startup, readiness and liveness probes on `GET /api/health`, port 8000.
- A Service on port 8000, and an Ingress when `ingress.main.enabled` is `true` (default class `nginx`, with a 100m body size annotation).
- PersistentVolumeClaims for the uploads and watch folders (`ReadWriteOnce`), kept after `helm uninstall`. Set `existingClaim` under `persistence.uploads` (or `persistence.watch`) to mount a claim you created instead. Rendering fails if uploads persistence is disabled while S3 storage is not enabled (use `type: emptyDir` to accept losing documents on restart), if a created claim has no size, or if `replicas` is above 1.
- A Secret named `<release>-auth` with a random initial `ADMIN_PASSWORD`, kept on upgrade and after `helm uninstall`.

What you must provide:

- **PostgreSQL.** The chart does not include a database. Pass `DATABASE_URL` (or the `POSTGRES_*` variables).

Example: create the database secret, and (optionally) your own uploads claim:

```bash
kubectl create namespace readur
kubectl -n readur create secret generic readur-db \
  --from-literal=DATABASE_URL='postgresql://readur:change-me@postgres.readur.svc:5432/readur'
kubectl -n readur apply -f - <<'EOF'
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: readur-uploads
spec:
  accessModes: ["ReadWriteOnce"]
  resources:
    requests:
      storage: 50Gi
EOF
```

A `values.yaml`:

```yaml
controllers:
  main:
    containers:
      readur:
        image:
          repository: ghcr.io/readur/readur
          tag: "3.1.3"
        env:
          DATABASE_URL:
            valueFrom:
              secretKeyRef:
                name: readur-db
                key: DATABASE_URL
          PUBLIC_URL: "https://readur.example.com"
          CONCURRENT_OCR_JOBS: "4"
        resources:
          requests:
            cpu: "1"
            memory: 1Gi
          limits:
            memory: 4Gi

persistence:
  uploads:
    existingClaim: readur-uploads

ingress:
  main:
    enabled: true
    className: nginx
    hosts:
      - host: readur.example.com
        paths:
          - path: /
            pathType: Prefix
            service:
              identifier: main
              port: http
    tls:
      - secretName: readur-tls
        hosts:
          - readur.example.com
```

Install from a checkout of the repository:

```bash
helm dependency build charts/readur
helm install readur charts/readur -n readur -f values.yaml
```

Read the generated admin password:

```bash
kubectl -n readur get secret readur-auth -o jsonpath='{.data.ADMIN_PASSWORD}' | base64 -d
```

Credentials:

- `JWT_SECRET` is optional: without it Readur generates a signing key and stores it in its database. To manage it yourself, set `auth.jwtSecret`, or put `JWT_SECRET` (at least 32 bytes) in the Secret named by `auth.existingSecret`.
- `auth.adminPassword` sets the initial admin password instead of a generated one; `auth.existingSecret` replaces the `<release>-auth` Secret entirely.
- **GitOps / `helm template`:** keeping the generated password across upgrades relies on Helm's `lookup` function, which only works when Helm talks to the cluster (`helm install` / `helm upgrade`). Renderers that do not, such as `helm template` and Argo CD, generate a new value on every render. With these tools, set `auth.existingSecret` to a Secret you manage (for example with Sealed Secrets or External Secrets), or set `auth.adminPassword`.

Keep one replica: see the note on running one instance per database under [Production Docker Compose](#production-docker-compose). The admin password reset command can be run in the pod:

```bash
kubectl -n readur exec deploy/readur -- /app/readur reset-admin-password
```

## Security Considerations

### Production Checklist

- [ ] Either leave `JWT_SECRET` unset (a key is generated and stored in the database; rotate it with `readur rotate-jwt-secret`) or set it to a random value of at least 32 bytes (`openssl rand -hex 32`)
- [ ] Change the initial admin password and delete the generated `initial-admin-password` file (if `ADMIN_PASSWORD` was not set)
- [ ] Keep `ALLOW_REGISTRATION` off unless you need self-registration (new accounts still need an administrator's approval)
- [ ] Never set `READUR_INSECURE_DEV_MODE` in production
- [ ] Set `PUBLIC_URL` behind a reverse proxy, and `TRUSTED_PROXIES` if the proxy is not on a loopback or private address (or `none` if clients connect directly from a private network)
- [ ] Set `METRICS_TOKEN` if Prometheus scrapes `/metrics`
- [ ] Serve Readur over HTTPS
- [ ] Do not publish the PostgreSQL port; use a strong database password
- [ ] Use a pinned image tag and update it deliberately
- [ ] Back up the database and the upload directory (see [Backup Strategy](#backup-strategy))
- [ ] Leave `LOCAL_SOURCE_ALLOWED_PATHS` unset (local folder sources admin-only) unless users need them, and then list only the directories they may use
- [ ] Keep secrets in a `.env` file with mode `600`, Docker secrets or Kubernetes Secrets rather than in committed files

## Next Steps

- Configure [monitoring](health-monitoring-guide.md)
- Review [security best practices](security-guide.md)
- Set up [automated backups](#backup-strategy)
- Tune [performance](performance-tuning.md)
