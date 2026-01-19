# Self-Hosting Quick Start

Get Readur running on your own server in 15 minutes. This guide covers the essential steps for a basic self-hosted deployment.

## Prerequisites

Before starting, ensure you have:

- **Linux server**: Ubuntu 20.04+, Debian 11+, or similar distribution
- **Docker Engine 24.0+**: [Installation guide](https://docs.docker.com/engine/install/)
- **Docker Compose v2+**: Usually included with Docker Engine; verify with `docker compose version`
- **System resources**:
  - 4GB RAM minimum (8GB+ recommended for OCR processing)
  - 20GB free disk space (more for large document libraries)
  - 2+ CPU cores recommended
- **Network access**: Ability to open port 8000 (or your chosen port) for web access
- **Domain name**: Optional, but recommended for HTTPS access

**Note**: This guide uses `docker compose` (Docker Compose v2). If you have the older standalone version, replace `docker compose` with `docker-compose`.

## Step 1: Choose Your Installation Method

Readur offers two installation methods:

| Method | Best For | Updates |
|--------|----------|---------|
| **Official Container** (Recommended) | Most users | Pull new image |
| **Build from Source** | Development, customization | Rebuild from git |

### Option A: Official Docker Container (Recommended)

Use the pre-built container from GitHub Container Registry for the fastest setup:

```bash
# Create a directory for Readur
mkdir readur && cd readur

# Download the docker-compose file for the official image
curl -O https://raw.githubusercontent.com/readur/readur/main/docker-compose.official.yml

# Rename for convenience
mv docker-compose.official.yml docker-compose.yml
```

Or create `docker-compose.yml` manually:

```yaml
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: readur
      POSTGRES_PASSWORD: readur
      POSTGRES_DB: readur
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U readur"]
      interval: 10s
      timeout: 5s
      retries: 5

  readur:
    image: ghcr.io/readur/readur:latest
    environment:
      DATABASE_URL: postgresql://readur:readur@postgres/readur
      JWT_SECRET: ${JWT_SECRET:-change-this-in-production}
      SERVER_HOST: 0.0.0.0
      SERVER_PORT: 8000
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

volumes:
  postgres_data:
```

### Option B: Build from Source

Clone the repository if you need to customize Readur or contribute to development:

```bash
git clone https://github.com/readur/readur.git
cd readur
```

## Step 2: Configure Environment

Create your environment file:

**For Option A (Official Container):**

```bash
# Create .env file with your secrets
cat > .env << 'EOF'
JWT_SECRET=your-secret-key-change-this
ADMIN_PASSWORD=YourSecurePassword123!
EOF
```

**For Option B (Build from Source):**

```bash
cp .env.example .env
```

Open `.env` in your preferred editor and configure these essential settings:

```bash
# Database connection (default works with Docker Compose)
DATABASE_URL=postgresql://readur:readur@postgres/readur

# Security - IMPORTANT: Change this in production!
# Generate a secure key with: openssl rand -hex 32
JWT_SECRET=your-super-secret-jwt-key-change-this-in-production

# Server binding
SERVER_HOST=0.0.0.0
SERVER_PORT=8000

# OCR settings
OCR_LANGUAGE=eng
CONCURRENT_OCR_JOBS=4
```

### Admin Password

By default, Readur auto-generates a secure admin password on first startup and displays it once in the logs. You can also set it explicitly in `.env`:

```bash
# Optional: Set a custom admin password (minimum 8 characters)
ADMIN_PASSWORD=YourSecurePassword123!
```

**Tip**: For production deployments, we recommend letting Readur auto-generate the password and saving it immediately from the startup logs.

## Step 3: Start Services

Launch Readur with Docker Compose:

**For Option A (Official Container):**

```bash
docker compose up -d
```

This pulls the pre-built image and starts immediately.

**For Option B (Build from Source):**

```bash
docker compose up -d --build
```

This builds the image locally (first run takes 3-5 minutes).

**Both options:**

1. Start the PostgreSQL database
2. Run database migrations automatically
3. Start the Readur application

Monitor the startup process:

```bash
docker compose logs -f readur
```

Look for these indicators of successful startup:

- `Database migrations completed successfully`
- `Server listening on 0.0.0.0:8000`

**First-time startup**: If you did not set `ADMIN_PASSWORD`, look for a line like:

```
READUR ADMIN USER CREATED - Password: AbCdEf123456...
```

**Save this password immediately** - it is only displayed once!

Press `Ctrl+C` to stop following the logs once startup is complete.

## Step 4: Verify Installation

Before proceeding, verify everything is running correctly:

```bash
# Check all services are running
docker compose ps
```

You should see both `readur` and `postgres` with status `running` or `healthy`.

Test the health endpoint:

```bash
curl http://localhost:8000/api/health
```

Expected response: `{"status":"ok"}` or similar JSON indicating healthy status.

**Storage locations on your host machine:**

| Location | Purpose |
|----------|---------|
| `./readur_uploads/` | Processed documents storage |
| `./readur_watch/` | Drop files here for automatic import |

These directories are created automatically when Readur starts.

## Step 5: Access Readur

Open your web browser and navigate to:

- **From the same machine**: http://localhost:8000
- **From your network**: http://your-server-ip:8000

You should see the Readur login page.

**Login credentials:**

- Username: `admin`
- Password: The password you set in `ADMIN_PASSWORD`, or the auto-generated password from the startup logs

**Congratulations!** You now have Readur running. Continue to the next step to upload your first document.

## Step 6: Initial Setup

### Upload Your First Document

1. Click **Upload** in the top navigation
2. Select PDF or image files
3. Enable **Process with OCR** for scanned documents
4. Click **Upload Files**

Watch as Readur processes the document and extracts text automatically.

### Configure OCR Languages

1. Go to **Settings** > **OCR Configuration**
2. Select your primary languages
3. Adjust `CONCURRENT_OCR_JOBS` in `.env` based on server capacity

### Set Up Automatic Import (Optional)

Readur can automatically import files placed in a watch folder.

**Option 1: Use the default watch folder**

Simply copy files to the `./readur_watch/` directory on your host machine:

```bash
cp ~/my-document.pdf ./readur_watch/
```

Readur checks for new files every 30 seconds by default.

**Option 2: Configure via the web interface**

1. Navigate to **Settings** > **Sources**
2. Add or modify the watch folder configuration
3. Enable **Auto-process** and **OCR** as needed

## Next Steps

### Security Hardening

For production deployments, implement these security measures:

1. **Enable HTTPS**: Set up a reverse proxy with TLS certificates. See [Reverse Proxy Setup](../REVERSE_PROXY.md)

2. **Use strong secrets**: Ensure `JWT_SECRET` is a long, random string:
   ```bash
   openssl rand -hex 32
   ```

3. **Configure firewall**: Only expose necessary ports (typically just 80/443 through your reverse proxy)

4. **Enable SSO** (optional): Set up [OIDC Authentication](../oidc-setup.md) for enterprise environments

### Performance Tuning

Adjust these settings in `.env` based on your hardware:

```bash
# Increase for faster OCR processing (uses more CPU/memory)
CONCURRENT_OCR_JOBS=4

# Memory limit per OCR job
MEMORY_LIMIT_MB=512

# Maximum file sizes
MAX_FILE_SIZE_MB=50
MAX_PDF_SIZE_MB=100
```

See [Performance Tuning Guide](../performance-tuning.md) for detailed recommendations.

### Backup Configuration

Create a backup script for your Readur data:

```bash
#!/bin/bash
# backup-readur.sh

BACKUP_DIR="/path/to/backups"
DATE=$(date +%Y%m%d_%H%M%S)

# Backup database
docker compose exec -T postgres pg_dump -U readur readur > "$BACKUP_DIR/readur-db-$DATE.sql"

# Backup uploaded documents
tar -czf "$BACKUP_DIR/readur-uploads-$DATE.tar.gz" ./readur_uploads

echo "Backup completed: $DATE"
```

Make it executable and schedule with cron:

```bash
chmod +x backup-readur.sh
```

See [Backup & Recovery Guide](../backup-recovery.md) for automated backup solutions.

### Updating Readur

#### Manual Updates

**For Option A (Official Container):**

```bash
# Pull the latest image
docker compose pull

# Restart with the new image
docker compose up -d
```

**For Option B (Build from Source):**

```bash
# Pull latest changes
git pull origin main

# Rebuild and restart
docker compose up -d --build
```

Check the [release notes](https://github.com/readur/readur/releases) for breaking changes before updating.

#### Automatic Updates with Watchtower

[Watchtower](https://containrrr.dev/watchtower/) can automatically update your Readur container when new versions are released.

**Add Watchtower to your `docker-compose.yml`:**

```yaml
  watchtower:
    image: containrrr/watchtower
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
    environment:
      # Check for updates daily at 4 AM
      WATCHTOWER_SCHEDULE: "0 0 4 * * *"
      # Only update containers with the label
      WATCHTOWER_LABEL_ENABLE: "true"
      # Clean up old images
      WATCHTOWER_CLEANUP: "true"
      # Optional: Send notifications
      # WATCHTOWER_NOTIFICATIONS: slack
      # WATCHTOWER_NOTIFICATION_SLACK_HOOK_URL: https://hooks.slack.com/...
    restart: unless-stopped
```

**Add the Watchtower label to your Readur service:**

```yaml
  readur:
    image: ghcr.io/readur/readur:latest
    labels:
      - "com.centurylinklabs.watchtower.enable=true"
    # ... rest of configuration
```

**Or run Watchtower standalone:**

```bash
docker run -d \
  --name watchtower \
  -v /var/run/docker.sock:/var/run/docker.sock \
  containrrr/watchtower \
  --schedule "0 0 4 * * *" \
  --cleanup \
  readur
```

This checks for updates daily at 4 AM and automatically restarts Readur with the new image.

**Watchtower configuration options:**

| Option | Description |
|--------|-------------|
| `WATCHTOWER_SCHEDULE` | Cron expression for update checks (default: every 24h) |
| `WATCHTOWER_CLEANUP` | Remove old images after updating |
| `WATCHTOWER_LABEL_ENABLE` | Only update labeled containers |
| `WATCHTOWER_NOTIFICATIONS` | Send notifications (email, slack, etc.) |

See the [Watchtower documentation](https://containrrr.dev/watchtower/) for more options.

## Troubleshooting

### Container Won't Start

Check the logs for specific errors:

```bash
docker compose logs readur
docker compose logs postgres
```

**Common causes:**

- Port 8000 already in use (see "Port Conflict" below)
- Insufficient memory
- Database connection failed

### Port Conflict

If port 8000 is already in use:

```bash
# Check what is using port 8000
ss -tlnp | grep 8000
```

**Solution**: Either stop the conflicting service, or change Readur's port in `.env`:

```bash
SERVER_PORT=8080
```

Then update the port mapping in `docker-compose.yml` (`"8080:8080"`) and restart:

```bash
docker compose up -d
```

### Database Connection Errors

If you see "connection refused" or "database does not exist":

```bash
# Verify PostgreSQL is running and healthy
docker compose ps postgres

# Check PostgreSQL logs
docker compose logs postgres
```

**Solution**: Wait for PostgreSQL to fully start (check for "database system is ready"), then restart Readur:

```bash
docker compose restart readur
```

### Permission Denied Errors

If uploads or watch folder operations fail:

```bash
# Check directory ownership
ls -la ./readur_uploads ./readur_watch

# Fix permissions if needed (Linux)
sudo chown -R 1000:1000 ./readur_uploads ./readur_watch
```

### OCR Not Processing

Verify OCR functionality:

```bash
# Check Readur logs for OCR-related messages
docker compose logs readur | grep -i ocr
```

**Common causes:**

- `CONCURRENT_OCR_JOBS=0` in your `.env` file
- Insufficient memory for OCR processing
- Unsupported file format

### Cannot Access from Browser

1. **Check the service is running:**
   ```bash
   docker compose ps
   ```

2. **Verify the port is listening:**
   ```bash
   ss -tlnp | grep 8000
   ```

3. **Check firewall settings:**
   ```bash
   # Ubuntu/Debian with UFW
   sudo ufw allow 8000/tcp

   # RHEL/CentOS with firewalld
   sudo firewall-cmd --add-port=8000/tcp --permanent
   sudo firewall-cmd --reload
   ```

4. **Test local connectivity:**
   ```bash
   curl http://localhost:8000/api/health
   ```

### Forgot Admin Password

If you lost the auto-generated admin password:

```bash
# Reset the admin password
docker compose exec readur readur reset-admin-password
```

This will generate and display a new password.

## Related Documentation

- [Complete Self-Hosting Guide](../self-hosting/index.md) - Comprehensive setup and configuration
- [S3 Storage Guide](../s3-storage-guide.md) - Cloud storage configuration
- [Performance Tuning](../performance-tuning.md) - Optimize for your hardware
- [Backup & Recovery](../backup-recovery.md) - Protect your data
- [Reverse Proxy Setup](../REVERSE_PROXY.md) - HTTPS and domain configuration
