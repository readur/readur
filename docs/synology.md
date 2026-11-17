# Synology NAS Deployment Guide

This guide covers running Readur on a Synology NAS with DSM 7 and Container Manager (called Docker on DSM versions before 7.2).

## Prerequisites

- A Synology NAS with an x86-64 (amd64) or 64-bit ARM (arm64) CPU. The Readur image is published for `linux/amd64` and `linux/arm64` only, so models with a 32-bit ARM CPU cannot run it.
- Container Manager installed from Package Center.
- SSH access (Control Panel > Terminal & SNMP) for reading the initial password, backups and troubleshooting.

## Quick Start

1. In File Station, create the folders `docker/readur/uploads` and `docker/readur/watch` (on `volume1` these are `/volume1/docker/readur/uploads` and `/volume1/docker/readur/watch`).
2. In Container Manager, create a project with the compose file below.
3. Open `http://<nas-ip>:8000` and sign in as `admin` with the initial password (see [Step 4](#step-4-sign-in)).

## Recommended Docker Compose Configuration

```yaml
services:
  readur:
    image: ghcr.io/readur/readur:latest
    container_name: readur
    ports:
      - "8000:8000"
    environment:
      # Readur reaches PostgreSQL over the project's Docker network,
      # by service name and container port.
      DATABASE_URL: postgresql://readur:readur@postgres:5432/readur

      UPLOAD_PATH: /app/uploads
      WATCH_FOLDER: /app/watch

      # Watch folder
      ALLOWED_FILE_TYPES: pdf,png,jpg,jpeg,tiff,bmp,gif,txt,doc,docx
      WATCH_INTERVAL_SECONDS: "30"
      FILE_STABILITY_CHECK_MS: "1000"

      # OCR
      OCR_LANGUAGE: eng
      CONCURRENT_OCR_JOBS: "2"
      OCR_TIMEOUT_SECONDS: "300"
      MAX_FILE_SIZE_MB: "50"
    volumes:
      - /volume1/docker/readur/uploads:/app/uploads
      - /volume1/docker/readur/watch:/app/watch:ro
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

  postgres:
    image: postgres:18
    container_name: readur-postgres
    environment:
      POSTGRES_USER: readur
      POSTGRES_PASSWORD: readur
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
    labels:
      # Keep auto-update tools away from the database
      - "wud.trigger.exclude=docker.autoupdate"
      - "com.centurylinklabs.watchtower.monitor-only=true"

volumes:
  postgres_data:
```

Change the database password (in both `POSTGRES_PASSWORD` and `DATABASE_URL`) before the first start; the postgres image applies it only when it creates the database. For a fixed version, replace `latest` with a release tag such as `3.1.3`.

`JWT_SECRET` is not needed: Readur generates a signing key on first start and stores it in the database.

## Understanding the Configuration

### Port Mapping

| Service | Container port | Host port | Notes |
|---------|---------------|-----------|-------|
| Readur | 8000 | 8000 | Web interface and API |
| PostgreSQL | 5432 | not published | Only Readur needs it, over the project network |

DSM runs its own PostgreSQL for some packages, so host port 5432 may be taken. The database does not need a host port at all. If you want to reach it from another computer (for example with pgAdmin), publish it on a free port such as `"5433:5432"`; `DATABASE_URL` stays on `postgres:5432` either way, because containers talk to each other on container ports.

### Volume Mounts

| Mount | Purpose | Notes |
|-------|---------|-------|
| `/volume1/docker/readur/uploads` | Stored documents, thumbnails, the initial admin password file | Must be writable |
| `/volume1/docker/readur/watch` | Watch folder: files put here are imported | Mounted read-only (`:ro`); Readur never modifies or deletes them |
| `postgres_data` | Database | Named volume |

### Why a Named Volume for PostgreSQL?

PostgreSQL needs to own its data directory and set strict permissions on it, which DSM's shared-folder permissions can prevent on a host path (`initdb: could not change permissions of directory`). A named volume is managed by Docker and avoids this.

With `postgres:18` the volume is mounted at `/var/lib/postgresql` with `PGDATA` set, as in the repository's `docker-compose.yml`. If you already run an older major version (for example `postgres:16`, mounted at `/var/lib/postgresql/data`), keep that version and mount point; moving to a new major version needs the [PostgreSQL Upgrade Guide](postgres-upgrade.md).

## Deployment Steps

### Step 1: Create Directories

In File Station, open the `docker` shared folder (create it if it does not exist), create `readur`, and inside it `uploads` and `watch`. Or over SSH:

```bash
mkdir -p /volume1/docker/readur/uploads /volume1/docker/readur/watch
```

### Step 2: Permissions

The Readur container runs as root, so it can write to `uploads` without changing the folder's permissions. If you add `user:` to the `readur` service, give that user write access to `uploads` and read access to `watch`.

### Step 3: Deploy via Container Manager

1. Open **Container Manager**, go to **Project** and click **Create**.
2. Name: `readur`; path: `/volume1/docker/readur`.
3. Source: create a `docker-compose.yml` and paste the configuration above.
4. Finish the wizard; the project is built and started.

### Step 4: Sign In

1. Wait until both containers are running and `readur` is healthy.
2. Read the generated admin password over SSH:

    ```bash
    sudo cat /volume1/docker/readur/uploads/.readur/initial-admin-password
    ```

3. Open `http://<nas-ip>:8000`, sign in as `admin`, change the password (Settings > Account), and delete the file.

To choose the initial password yourself, set `ADMIN_PASSWORD` in the `readur` environment before the first start. Self-registration is off; create further accounts in Settings > Users.

## Synology-Specific Considerations

| Issue | Solution |
|-------|----------|
| Port 5432 already in use | Do not publish the PostgreSQL port, or publish it as `5433:5432` |
| Permission errors on PostgreSQL data | Use a named volume instead of a host path |
| PostgreSQL broken after an automatic update | Pin the major version and exclude the container from auto-updates (see below) |
| Slow processing or high memory use | Lower `CONCURRENT_OCR_JOBS` (see [Performance](#performance)) |
| HTTPS | Use DSM's reverse proxy (Control Panel > Login Portal > Advanced > Reverse Proxy) to forward an HTTPS host name to `http://localhost:8000`, and set `PUBLIC_URL` to that address. Add the WebSocket headers in the rule's custom headers so live sync progress works |

## Troubleshooting

### PostgreSQL Permission Error

```text
initdb: error: could not change permissions of directory "/var/lib/postgresql/data"
```

The PostgreSQL data is on a host path. Use the named volume from the configuration above.

### PostgreSQL Fails After an Update

If you use an auto-update tool such as What's Up Docker or Watchtower, or an unpinned `postgres` tag, PostgreSQL may have been moved to a new major version. A data directory only works with the major version that created it.

Set the `postgres` image back to the major version that created the data (for example `postgres:16`) and start the project again; the data is intact. The `labels` in the configuration above tell What's Up Docker and Watchtower to leave the database container alone.

If the image itself seems damaged (for example `exec: postgres: not found`), pull it again; this does not touch the data volume:

```bash
cd /volume1/docker/readur
sudo docker compose down
sudo docker pull postgres:18
sudo docker compose up -d
```

Never add `-v` to `docker compose down`: it deletes the named volume and with it the database.

### Container Keeps Restarting

```bash
sudo docker logs readur
sudo docker logs readur-postgres
```

The last lines of the Readur log name the problem (database connection, migrations, storage); see [Troubleshooting](troubleshooting.md#startup-problems).

### Cannot Connect to the Web Interface

1. Check that the container is running: `sudo docker ps`.
2. Check that the DSM firewall allows port 8000.
3. From an SSH session on the NAS, try `curl http://localhost:8000/api/health`; it returns `{"status":"ok"}` when Readur is up.

### Database Connection Fails

1. Check that `readur-postgres` is healthy: `sudo docker ps`.
2. `DATABASE_URL` must use the service name `postgres` and port 5432, not `localhost` or the published host port.
3. The user and password must match what the database was created with; see `sudo docker logs readur-postgres`.

## Performance

`CONCURRENT_OCR_JOBS` sets how many documents are OCRed at the same time; each running job keeps about one CPU core busy.

- 2-core NAS or less than 4 GB of RAM: `1`
- 4-core NAS with 4 GB or more: `2` to `4`

Do not go above the number of cores. `MEMORY_LIMIT_MB` and `CPU_PRIORITY` do not limit anything; to cap Readur's resources, set limits on the container in Container Manager. See [Performance Tuning](performance-tuning.md).

## Updating Readur

With the `latest` tag:

```bash
cd /volume1/docker/readur
sudo docker compose pull readur
sudo docker compose up -d
```

With a pinned tag, change the tag in `docker-compose.yml` first. Database migrations run automatically when the new version starts. Take a backup before updating.

## Backing Up

### Database Backup

```bash
sudo docker exec readur-postgres pg_dump -U readur -Fc readur > /volume1/docker/readur/readur.dump
```

Restore it with `pg_restore` into an empty database, with Readur stopped (see [Backup Strategy](deployment.md#backup-strategy)).

### Full Backup

Back up together:

- the database dump (above);
- `/volume1/docker/readur/uploads`, which holds the stored documents;
- `/volume1/docker/readur/watch`, if you keep originals there.

Hyper Backup can include the `docker/readur` folder; run the database dump first (for example as a scheduled task in Control Panel > Task Scheduler) so that the dump is part of the backup.

## Further Reading

- [Deployment Guide](deployment.md) - General deployment information
- [Watch Folder Guide](WATCH_FOLDER.md) - Automatic import from a folder
- [PostgreSQL Upgrade Guide](postgres-upgrade.md) - Database version migrations
- [Backup and Recovery](backup-recovery.md) - Backup strategies
