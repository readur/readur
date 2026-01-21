# Synology NAS Deployment Guide

This guide covers deploying Readur on Synology NAS devices running DSM 7.x using Container Manager (Docker).

## Prerequisites

- Synology NAS with DSM 7.0 or later
- Container Manager package installed (available in Package Center)
- SSH access (optional, for troubleshooting)
- Basic familiarity with Docker and docker-compose

## Quick Start

1. Create directories via File Station:
   - `/volume1/docker/readur/uploads`
   - `/volume1/docker/readur/watch`

2. Create a new Project in Container Manager with the docker-compose configuration below

3. Access Readur at `http://your-nas-ip:8000`

## Recommended Docker Compose Configuration

```yaml
services:
  readur:
    image: ghcr.io/readur/readur:main
    container_name: readur
    ports:
      - "8000:8000"
    environment:
      # Database connection - uses internal Docker network port (5432)
      # Even though we expose 5433 externally, containers communicate internally on 5432
      DATABASE_URL: postgresql://readur:readur@postgres:5432/readur

      # Or use individual variables instead of DATABASE_URL:
      # POSTGRES_HOST: postgres
      # POSTGRES_PORT: 5432        # Internal Docker network port
      # POSTGRES_DB: readur
      # POSTGRES_USER: readur
      # POSTGRES_PASSWORD: readur

      # Security - CHANGE THIS in production
      JWT_SECRET: your-secret-key-change-this

      # File paths
      UPLOAD_PATH: /app/uploads
      WATCH_FOLDER: /app/watch

      # File processing
      ALLOWED_FILE_TYPES: pdf,png,jpg,jpeg,tiff,bmp,gif,txt,doc,docx
      WATCH_INTERVAL_SECONDS: 30
      FILE_STABILITY_CHECK_MS: 1000
      MAX_FILE_AGE_HOURS: 168

      # OCR settings
      OCR_LANGUAGE: eng
      CONCURRENT_OCR_JOBS: 2        # Reduce for lower-end NAS models
      OCR_TIMEOUT_SECONDS: 300
      MAX_FILE_SIZE_MB: 50

    volumes:
      - /volume1/docker/readur/uploads:/app/uploads
      - /volume1/docker/readur/watch:/app/watch:ro    # Read-only watch folder
    depends_on:
      postgres:
        condition: service_healthy
    restart: unless-stopped

  postgres:
    image: postgres:16.8-alpine    # Pin specific version to avoid update issues
    container_name: readur-postgres
    environment:
      POSTGRES_USER: readur
      POSTGRES_PASSWORD: readur
      POSTGRES_DB: readur
    ports:
      # IMPORTANT: Synology DSM may use port 5432 internally
      # We expose on 5433 to avoid conflicts
      # Format: HOST_PORT:CONTAINER_PORT
      # - Readur connects via Docker network on 5432 (container port)
      # - External tools (pgAdmin, etc.) connect via 5433 (host port)
      - "5433:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data  # Use named volume, not host path
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U readur"]
      interval: 10s
      timeout: 5s
      retries: 5
    restart: unless-stopped

volumes:
  postgres_data:    # Named volume avoids permission issues
```

## Understanding the Configuration

### Port Mapping

| Service | Container Port | Host Port | Notes |
|---------|---------------|-----------|-------|
| Readur | 8000 | 8000 | Web UI and API |
| PostgreSQL | 5432 | 5433 | Database (5433 avoids DSM conflict) |

**Why port 5433 for PostgreSQL?**

Synology DSM may use port 5432 internally for its own services. By mapping to 5433 on the host, we avoid conflicts while keeping the standard 5432 port inside the Docker network.

> **Important:** You do NOT need to change `DATABASE_URL` - Readur connects to PostgreSQL via the internal Docker network on port 5432. The 5433 mapping is only for external access (e.g., connecting with pgAdmin from your computer).

### Volume Mounts

| Mount | Purpose | Notes |
|-------|---------|-------|
| `/volume1/docker/readur/uploads` | Processed documents | Read/write access needed |
| `/volume1/docker/readur/watch` | Watch folder for new files | Mounted read-only (`:ro`) for safety |
| `postgres_data` | Database storage | Named volume (avoids permission issues) |

### Why Named Volumes for PostgreSQL?

Synology DSM has strict permission controls that can prevent PostgreSQL from initializing when using host path mounts. Named Docker volumes work around this issue because Docker manages the permissions internally.

## Deployment Steps

### Step 1: Create Directories

Using File Station:

1. Navigate to the `docker` shared folder (create it if it doesn't exist)
2. Create folder: `readur`
3. Inside `readur`, create: `uploads` and `watch`

Or via SSH:
```bash
mkdir -p /volume1/docker/readur/uploads
mkdir -p /volume1/docker/readur/watch
```

### Step 2: Set Permissions

Ensure the Docker user can read/write to these directories:

1. Right-click the `readur` folder in File Station
2. Select Properties > Permission
3. Add permission for "Everyone" with Read/Write access (or configure more restrictively as needed)

### Step 3: Deploy via Container Manager

1. Open **Container Manager** from DSM
2. Go to **Project** > **Create**
3. Name: `readur`
4. Path: `/volume1/docker/readur`
5. Source: Select "Create docker-compose.yml"
6. Paste the docker-compose configuration above
7. Click **Next** and then **Done**

### Step 4: Verify Deployment

1. Wait for containers to start (check Project status)
2. Access Readur at `http://your-nas-ip:8000`
3. Create an account and start uploading documents

## Synology-Specific Considerations

| Issue | Solution |
|-------|----------|
| **Port 5432 conflict** | DSM may use 5432 internally. Map to `5433:5432` instead. Readur still connects internally on 5432. |
| **Permission errors on postgres data** | Use named volumes (`postgres_data:`) instead of host paths |
| **"postgres: not found" error** | Pin specific version (e.g., `postgres:16.8-alpine`) and re-pull image |
| **Watch folder permissions** | Mount with `:ro` flag; create folders via File Station first |
| **External DB access** | Use port 5433 from Synology host (e.g., pgAdmin). Container-to-container uses 5432. |
| **Low memory NAS** | Reduce `CONCURRENT_OCR_JOBS` to 1 or 2 |
| **ARM-based NAS** | Most images support ARM64; verify container starts correctly |

## Troubleshooting

### PostgreSQL Permission Error

```
initdb: error: could not change permissions of directory "/var/lib/postgresql/data"
```

**Cause:** DSM's permission model prevents PostgreSQL from setting up its data directory on a host-mounted path.

**Solution:** Use a named Docker volume instead of a host path mount:

```yaml
volumes:
  - postgres_data:/var/lib/postgresql/data  # Correct
  # NOT: /volume1/docker/readur/postgres:/var/lib/postgresql/data
```

### "postgres: not found" Error

```
/usr/local/bin/docker-entrypoint.sh: exec: postgres: not found
```

**Cause:** Corrupted image download or version mismatch.

**Solution:** Remove and re-pull the postgres image:

```bash
# SSH into your Synology
ssh admin@your-nas-ip

# Stop the project
cd /volume1/docker/readur
docker compose down -v

# Remove and re-pull the image
docker rmi postgres:16.8-alpine
docker pull postgres:16.8-alpine

# Restart
docker compose up -d
```

### Container Keeps Restarting

**Check logs:**
```bash
docker logs readur
docker logs readur-postgres
```

**Common causes:**
- Database not ready (wait for healthcheck)
- Invalid environment variables
- Permission issues on mounted volumes

### Cannot Connect to Web UI

1. Verify container is running: `docker ps`
2. Check if port 8000 is blocked by DSM firewall
3. Try accessing via `http://localhost:8000` from SSH

### Database Connection Timeout

If Readur can't connect to PostgreSQL:

1. Ensure postgres container is healthy: `docker ps`
2. Verify DATABASE_URL uses `postgres` (service name), not `localhost`
3. Check postgres logs: `docker logs readur-postgres`

## Performance Tuning for Synology

### Low-End NAS (DS218, DS220j, DS223j)

```yaml
environment:
  CONCURRENT_OCR_JOBS: 1
  MEMORY_LIMIT_MB: 256
  MAX_FILE_SIZE_MB: 25
```

### Mid-Range NAS (DS220+, DS420+, DS720+)

```yaml
environment:
  CONCURRENT_OCR_JOBS: 2
  MEMORY_LIMIT_MB: 512
  MAX_FILE_SIZE_MB: 50
```

### High-End NAS (DS920+, DS1520+, DS1621+)

```yaml
environment:
  CONCURRENT_OCR_JOBS: 4
  MEMORY_LIMIT_MB: 1024
  MAX_FILE_SIZE_MB: 100
```

## Updating Readur

To update to a new version:

```bash
cd /volume1/docker/readur
docker compose pull
docker compose up -d
```

Or via Container Manager:
1. Go to Project > readur
2. Click **Action** > **Build**
3. Enable "Pull image" and click **Build**

## Backing Up

### Database Backup

```bash
docker exec -t readur-postgres pg_dumpall -U readur > /volume1/docker/readur/backup.sql
```

### Full Backup

Back up these locations:
- `/volume1/docker/readur/uploads` - Your documents
- `/volume1/docker/readur/watch` - Watch folder (if you store files there)
- Database dump (see above)

For PostgreSQL major version upgrades, see the [PostgreSQL Upgrade Guide](postgres-upgrade.md).

## Further Reading

- [Deployment Guide](deployment.md) - General deployment information
- [Watch Folder Guide](WATCH_FOLDER.md) - Configure automatic file processing
- [PostgreSQL Upgrade Guide](postgres-upgrade.md) - Database version migrations
- [Backup and Recovery](backup-recovery.md) - Backup strategies
