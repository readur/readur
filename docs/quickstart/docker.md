# Docker Quick Start

## Purpose

Deploy Readur using Docker in 5 minutes. This guide provides the fastest path to a working Readur installation using Docker Compose.

## Prerequisites

- Docker Engine 20.10+ installed
- Docker Compose 2.0+ installed
- 2GB RAM available for containers
- 10GB free disk space

## Step 1: Quick Deploy

Create a new directory and download the Docker Compose configuration:

```bash
mkdir readur && cd readur

# Download docker-compose.yml
curl -O https://raw.githubusercontent.com/readur/readur/main/docker-compose.yml

# Download environment template
curl -O https://raw.githubusercontent.com/readur/readur/main/.env.example
mv .env.example .env
```

## Step 2: Minimal Configuration

Edit `.env` with only essential settings:

```bash
# Required settings only
APP_SECRET_KEY=$(openssl rand -hex 32)
POSTGRES_PASSWORD=$(openssl rand -hex 16)
ADMIN_PASSWORD=changeme123

# Everything else uses secure defaults
```

## Step 3: Launch

Start all services:

```bash
docker-compose up -d
```

View startup progress:

```bash
docker-compose logs -f
```

## Step 4: Access Application

Once started (typically 30-60 seconds):

1. Open http://localhost:8000
2. Login with:
   - Username: `admin`
   - Password: `changeme123` (or what you set)

## Step 5: Test Document Processing

Upload a test document:

```bash
# Upload via API
curl -X POST http://localhost:8000/api/upload \
  -H "Authorization: Bearer your-token" \
  -F "file=@test.pdf" \
  -F "ocr=true"
```

Or use the web interface:
1. Click **Upload** button
2. Select files
3. Enable **OCR Processing**
4. Click **Upload**

## Docker Compose Services

Your deployment includes these containers:

```yaml
services:
  readur:         # Main application
  postgres:       # Database
  redis:          # Cache and queues
  ocr-worker:     # OCR processing
  nginx:          # Web server (optional)
```

## Container Management

### View Service Status

```bash
docker-compose ps
```

### Stop Services

```bash
docker-compose stop
```

### Remove Everything

```bash
docker-compose down -v  # Includes volumes (data loss!)
```

### Update Containers

```bash
docker-compose pull
docker-compose up -d
```

## Data Persistence

Docker volumes store your data:

```bash
# List volumes
docker volume ls

# Backup database
docker-compose exec postgres pg_dump -U readur > backup.sql

# Backup documents
docker run --rm -v readur_documents:/data -v $(pwd):/backup \
  alpine tar czf /backup/documents.tar.gz /data
```

## Resource Configuration

### Memory Limits

Add to `docker-compose.yml`:

```yaml
services:
  readur:
    mem_limit: 2g
    memswap_limit: 2g
    
  ocr-worker:
    mem_limit: 1g
    cpus: '2.0'
```

### Storage Locations

Configure volume mounts:

```yaml
volumes:
  documents:
    driver: local
    driver_opts:
      type: none
      device: /mnt/storage/readur
      o: bind
```

## Network Configuration

### Custom Port

Change the exposed port in `docker-compose.yml`:

```yaml
services:
  readur:
    ports:
      - "9000:8000"  # Access on port 9000
```

### Internal Network Only

Remove port exposure for internal use:

```yaml
services:
  readur:
    # ports:    # Commented out
    #   - "8000:8000"
    networks:
      - internal
```

## Troubleshooting

### Containers Keep Restarting

Check logs for each service:

```bash
docker-compose logs readur
docker-compose logs postgres
docker-compose logs ocr-worker
```

### Permission Errors

Fix volume permissions:

```bash
docker-compose exec readur chown -R readur:readur /data
```

### Port Already in Use

Change the port binding:

```bash
# In docker-compose.yml
ports:
  - "8080:8000"  # Use port 8080 instead
```

### Low Memory

Reduce OCR workers:

```bash
# In .env
OCR_WORKERS=1
OCR_MAX_PARALLEL=1
```

## Docker Commands Reference

```bash
# View logs
docker-compose logs -f [service]

# Execute commands in container
docker-compose exec readur bash

# Restart single service
docker-compose restart ocr-worker

# Check resource usage
docker stats

# Clean up unused resources
docker system prune -a
```

## Next Steps

### Production Deployment

For production use:
1. [Configure HTTPS](../self-hosting/reverse-proxy.md)
2. [Set up backups](../self-hosting/backup.md)
3. [Enable monitoring](../health-monitoring-guide.md)
4. [Configure authentication](../self-hosting/authentication.md)

### Scaling

Handle more documents:
1. [Optimize OCR processing](../self-hosting/performance.md)
2. [Use S3 storage](../self-hosting/storage.md)
3. [Increase server resources](../self-hosting/performance.md)

## Related Documentation

- [Self-Hosting Guide](../self-hosting/index.md) - Complete deployment guide
- [Configuration Reference](../configuration-reference.md) - All configuration options
- [Container Architecture](../architecture.md) - How services interact
- [Troubleshooting Guide](../troubleshooting.md) - Common issues and solutions