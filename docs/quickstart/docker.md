# Docker Quick Start

Run Readur from the published Docker image, `ghcr.io/readur/readur`, without building anything. A Readur installation is two containers: the Readur server (web interface, API, OCR and background processing in one process) and PostgreSQL.

## What You Need

Docker with the Compose plugin (`docker compose version` should work), and enough CPU and memory for OCR, which is the heaviest work Readur does.

## Step 1: Create the Compose File

Create a directory for Readur and a `docker-compose.yml` in it:

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
      ADMIN_PASSWORD: ${ADMIN_PASSWORD:-}
      UPLOAD_PATH: /app/uploads
      WATCH_FOLDER: /app/watch
      OCR_LANGUAGE: eng
      CONCURRENT_OCR_JOBS: 2
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

`latest` is the most recent release. You can pin a release version such as `2.6.0`, or use `main` for builds of the main branch.

## Step 2: Set the Passwords

Create a `.env` file next to `docker-compose.yml`; Docker Compose substitutes its values into the file above:

```bash
cat > .env << EOF
POSTGRES_PASSWORD=$(openssl rand -hex 16)
ADMIN_PASSWORD=choose-a-long-password
EOF
```

`POSTGRES_PASSWORD` is used when the database is first created. `ADMIN_PASSWORD` (at least 8 characters) becomes the password of the `admin` account created on first startup. Leave it out and Readur generates a random password instead (see Step 4).

`JWT_SECRET` is not needed: without it Readur generates a signing key on first start and stores it in the database. See the [Configuration Guide](../getting-started/configuration.md) for all other settings.

## Step 3: Start Readur

```bash
docker compose up -d
docker compose logs -f readur
```

Readur runs its database migrations and then logs `✅ HTTP server bound to: 0.0.0.0:8000`. Press `Ctrl+C` to stop following the logs.

## Step 4: Sign In

Open `http://localhost:8000` and sign in as `admin` with your `ADMIN_PASSWORD`.

If you did not set `ADMIN_PASSWORD`, read the generated password (it is not written to the logs):

```bash
docker compose exec readur cat /app/uploads/.readur/initial-admin-password
```

After signing in and changing the password, delete the file.

## Step 5: Test Document Processing

Open **Intake** → **Upload** and drop a PDF, image or text file onto the upload area. The document appears in the **Library** and is queued for OCR; when OCR has finished, search for a word you know is in it.

To test the API instead, log in for a session token and upload a file:

```bash
token=$(curl -s -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"your-admin-password"}' | jq -r .token)

curl -X POST http://localhost:8000/api/documents \
  -H "Authorization: Bearer $token" \
  -F "file=@test.pdf"
```

## Managing Your Installation

### Status

```bash
docker compose ps
curl http://localhost:8000/api/health
```

The health endpoint answers `{"status":"ok"}`.

### Stopping and Starting

```bash
docker compose stop
docker compose start
```

### Updating

```bash
docker compose pull
docker compose up -d
```

Migrations for the new version run automatically at startup. Read the [release notes](https://github.com/readur/readur/releases) before updating.

### Complete Removal

```bash
docker compose down -v
rm -rf ./readur_uploads ./readur_watch
```

`down -v` deletes the database volume, and removing the directories deletes your documents. Only do this if you want to start fresh.

## Backing Up Your Data

Documents are in `./readur_uploads` and everything else is in the PostgreSQL database. Back up both:

```bash
# Database
docker compose exec -T postgres pg_dump -U readur readur > readur-db.sql

# Documents
tar czf readur-uploads.tar.gz ./readur_uploads
```

See the [Backup Guide](../self-hosting/backup.md).

## Tuning

### Resource Limits

Readur does all its work, OCR included, in the `readur` container. To cap its resources, use Compose limits:

```yaml
services:
  readur:
    mem_limit: 2g
    cpus: 2.0
```

Keep `CONCURRENT_OCR_JOBS` in line with the CPUs and memory you give the container.

### Storage Location

Change the host side of the volume mounts to store documents elsewhere, for example on a dedicated disk:

```yaml
services:
  readur:
    volumes:
      - /mnt/storage/readur/uploads:/app/uploads
      - /mnt/storage/readur/watch:/app/watch
```

To store documents in S3 instead, see the [S3 Storage Guide](../s3-storage-guide.md).

### Watch Folder

Files copied to `./readur_watch` are imported automatically once they have stopped changing. Only extensions in `ALLOWED_FILE_TYPES` (default `pdf,txt,doc,docx,png,jpg,jpeg`) are imported. See [Watch Folder](../WATCH_FOLDER.md).

## Network Configuration

### Custom Port

Change the host side of the port mapping; Readur keeps listening on `8000` inside the container:

```yaml
services:
  readur:
    ports:
      - "9000:8000"
```

### Behind a Reverse Proxy

Bind the port to the loopback interface only, so that Readur is reachable only through the proxy, and set `PUBLIC_URL` to the address users open:

```yaml
services:
  readur:
    ports:
      - "127.0.0.1:8000:8000"
    environment:
      PUBLIC_URL: https://readur.example.com
```

See the [Reverse Proxy Guide](../REVERSE_PROXY.md).

## Troubleshooting

### Containers Keep Restarting

```bash
docker compose logs readur
docker compose logs postgres
```

Readur prints the configuration it loaded at startup, followed by any error that stopped it.

### Database Authentication Fails

PostgreSQL applies `POSTGRES_PASSWORD` only when it first creates the database. If you changed the password afterwards, either change it back or delete the `postgres_data` volume (this deletes the database).

### Port Already in Use

Map another host port, as in [Custom Port](#custom-port).

### OCR Is Slow or Runs Out of Memory

Lower `CONCURRENT_OCR_JOBS` and restart with `docker compose up -d`.

## Docker Commands Reference

```bash
# Follow the logs
docker compose logs -f readur

# Open a shell in the Readur container
docker compose exec readur bash

# Reset the admin password
docker compose exec readur /app/readur reset-admin-password

# Restart Readur
docker compose restart readur

# Resource usage
docker stats
```

## Next Steps

### Production Deployment

1. [Configure HTTPS](../self-hosting/reverse-proxy.md)
2. [Set up backups](../self-hosting/backup.md)
3. [Enable monitoring](../health-monitoring-guide.md)
4. [Configure authentication](../self-hosting/authentication.md)

### Scaling

1. [Optimize OCR processing](../self-hosting/performance.md)
2. [Use S3 storage](../self-hosting/storage.md)

## Related Documentation

- [Self-Hosting Guide](../self-hosting/index.md) - Complete deployment guide
- [Configuration Reference](../configuration-reference.md) - All configuration options
- [Architecture](../architecture.md) - How Readur is built
- [Troubleshooting Guide](../troubleshooting.md) - Common issues and solutions
