# Deployment Guide

This guide covers production deployment strategies, SSL setup, monitoring, backups, and best practices for running Readur in production.

> 🆕 **New in 2.5.4**: S3 storage backend support! See the [Migration Guide](migration-guide.md) to migrate from local storage to S3, and the [S3 Storage Guide](s3-storage-guide.md) for complete setup instructions.

## Table of Contents

- [Production Docker Compose](#production-docker-compose)
- [Volume Path Syntax](#volume-path-syntax)
- [Platform-Specific Deployment Notes](#platform-specific-deployment-notes)
  - [Synology NAS](synology.md) - Complete Synology deployment guide
- [Network Filesystem Mounts](#network-filesystem-mounts)
  - [NFS Mounts](#nfs-mounts)
  - [SMB/CIFS Mounts](#smbcifs-mounts)
  - [S3 Mounts](#s3-mounts)
- [SSL/HTTPS Setup](#sslhttps-setup)
  - [Nginx Configuration](#nginx-configuration)
  - [Traefik Configuration](#traefik-configuration)
- [Health Checks](#health-checks)
- [Backup Strategy](#backup-strategy)
- [Upgrading PostgreSQL](postgres-upgrade.md)
- [Monitoring](#monitoring)
- [Deployment Platforms](#deployment-platforms)
  - [Docker Swarm](#docker-swarm)
  - [Kubernetes](#kubernetes)
  - [Cloud Platforms](#cloud-platforms)
- [Security Considerations](#security-considerations)

## Production Docker Compose

For production deployments, create a custom `docker-compose.prod.yml`:

```yaml
services:
  readur:
    image: ghcr.io/readur/readur:main
    ports:
      - "8000:8000"
    environment:
      # Core Configuration
      - DATABASE_URL=postgresql://readur:${DB_PASSWORD}@postgres:5432/readur
      - JWT_SECRET=${JWT_SECRET}
      - SERVER_ADDRESS=0.0.0.0:8000
      
      # File Storage
      - UPLOAD_PATH=/app/uploads
      - WATCH_FOLDER=/app/watch
      - ALLOWED_FILE_TYPES=pdf,png,jpg,jpeg,tiff,bmp,gif,txt,doc,docx
      
      # Watch Folder Settings
      - WATCH_INTERVAL_SECONDS=30
      - FILE_STABILITY_CHECK_MS=500
      - MAX_FILE_AGE_HOURS=168
      
      # OCR Configuration
      - OCR_LANGUAGE=eng
      - CONCURRENT_OCR_JOBS=4
      - OCR_TIMEOUT_SECONDS=300
      - MAX_FILE_SIZE_MB=100
      
      # Performance Tuning
      - MEMORY_LIMIT_MB=1024
      - CPU_PRIORITY=normal
      - ENABLE_COMPRESSION=true
      
      # Threading Configuration (fixed values)
      - OCR_RUNTIME_THREADS=3      # OCR processing threads
      - BACKGROUND_RUNTIME_THREADS=2 # Background task threads
      - DB_RUNTIME_THREADS=2        # Database connection threads
    
    volumes:
      # Document storage
      - ./data/uploads:/app/uploads
      
      # Watch folder - mount your network drives here
      - /mnt/nfs/documents:/app/watch
      # or SMB: - /mnt/smb/shared:/app/watch
      # or S3: - /mnt/s3/bucket:/app/watch
    
    depends_on:
      - postgres
    restart: unless-stopped
    
    # Resource limits for production
    deploy:
      resources:
        limits:
          memory: 2G
          cpus: '2.0'
        reservations:
          memory: 512M
          cpus: '0.5'

  postgres:
    image: postgres:16.8-alpine
    environment:
      - POSTGRES_USER=readur
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=readur
      - POSTGRES_INITDB_ARGS=--encoding=UTF-8 --lc-collate=en_US.UTF-8 --lc-ctype=en_US.UTF-8
    
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./postgres-config:/etc/postgresql/conf.d:ro
    
    # PostgreSQL optimization for document search
    command: >
      postgres
      -c shared_buffers=256MB
      -c effective_cache_size=1GB
      -c max_connections=100
      -c default_text_search_config=pg_catalog.english
    
    restart: unless-stopped
    
    # Don't expose port in production
    # ports:
    #   - "5433:5432"

volumes:
  postgres_data:
    driver: local
```

Deploy with environment file:
```bash
# Create .env file with secrets
cat > .env << EOF
JWT_SECRET=$(openssl rand -base64 64)
DB_PASSWORD=$(openssl rand -base64 32)
EOF

# Deploy
docker compose -f docker-compose.prod.yml --env-file .env up -d
```

## Volume Path Syntax

Understanding Docker volume path syntax is critical for successful deployments, especially when using container management tools like Portainer or running on NAS devices.

### Path Types

| Syntax | Example | Description |
|--------|---------|-------------|
| `./path` | `./uploads:/app/uploads` | **Relative path** - relative to the docker-compose.yml file location |
| `/path` | `/DATA/uploads:/app/uploads` | **Absolute path** - absolute path on the host filesystem |
| `name:` | `postgres_data:/var/lib/postgresql/data` | **Named volume** - Docker-managed volume |

### Common Mistakes

**Mistake 1: Using relative path when absolute is needed**

```yaml
# WRONG - Creates ./DATA/AppData relative to compose file
volumes:
  - ./DATA/AppData/readur/uploads:/app/uploads

# CORRECT - Uses absolute path /DATA/AppData on host
volumes:
  - /DATA/AppData/readur/uploads:/app/uploads
```

**Mistake 2: Case sensitivity on Linux**

Linux filesystems are case-sensitive. `/DATA` and `/data` are different directories:

```yaml
# These are DIFFERENT paths on Linux:
- /DATA/uploads:/app/uploads   # Capital DATA
- /data/uploads:/app/uploads   # Lowercase data
```

**Mistake 3: Path doesn't exist**

Docker will attempt to create the host path, but this fails if:
- The parent directory doesn't exist
- Docker lacks write permission to create it
- The filesystem is read-only

```bash
# Create directories before starting containers
mkdir -p /DATA/AppData/readur/uploads
mkdir -p /DATA/AppData/readur/watch
```

### Recommended Approach

For most deployments, use **named volumes** for database storage and **bind mounts** for user-accessible directories:

```yaml
volumes:
  # Named volume for database (Docker manages location)
  postgres_data:

services:
  postgres:
    volumes:
      - postgres_data:/var/lib/postgresql/data  # Named volume

  readur:
    volumes:
      # Bind mounts for user-accessible directories
      - /path/to/your/uploads:/app/uploads      # Absolute path
      - /path/to/your/documents:/app/watch      # Absolute path
```

## Platform-Specific Deployment Notes

Different platforms have specific requirements for volume paths and Docker configuration.

### Portainer

When deploying via Portainer's Stacks feature:

1. **Use absolute paths** - Relative paths (starting with `./`) are resolved from Portainer's working directory, not where you expect
2. **Create directories first** - Portainer won't create host directories automatically
3. **Check the Volumes tab** - Verify volume mappings after deployment

```yaml
# Recommended for Portainer deployments
services:
  readur:
    volumes:
      - /srv/readur/uploads:/app/uploads
      - /srv/readur/watch:/app/watch
```

### ZimaOS

ZimaOS uses `/DATA/AppData/` as the standard location for application data:

```yaml
services:
  readur:
    volumes:
      - /DATA/AppData/readur/uploads:/app/uploads
      - /DATA/AppData/readur/watch:/app/watch

  postgres:
    volumes:
      - /DATA/AppData/readur/postgres_data:/var/lib/postgresql/data
```

**Important:** Use `/DATA` (absolute), not `./DATA` (relative).

### Unraid

Unraid uses `/mnt/user/appdata/` for container data:

```yaml
services:
  readur:
    volumes:
      - /mnt/user/appdata/readur/uploads:/app/uploads
      - /mnt/user/appdata/readur/watch:/app/watch

  postgres:
    volumes:
      - /mnt/user/appdata/readur/postgres:/var/lib/postgresql/data
```

### Synology DSM

Synology NAS devices require special configuration due to DSM's permission model and potential port conflicts.

**Key considerations:**
- Use port `5433:5432` for PostgreSQL to avoid DSM internal conflicts
- Use named Docker volumes for PostgreSQL data (avoids permission errors)
- Pin specific PostgreSQL versions (e.g., `postgres:16.8-alpine`)
- Create directories via File Station before deploying

For complete setup instructions, troubleshooting, and performance tuning, see the **[Synology Deployment Guide](synology.md)**.

### QNAP

QNAP NAS devices typically use:

```yaml
services:
  readur:
    volumes:
      - /share/Container/readur/uploads:/app/uploads
      - /share/Container/readur/watch:/app/watch
```

### TrueNAS SCALE

TrueNAS SCALE uses dataset paths:

```yaml
services:
  readur:
    volumes:
      - /mnt/pool/apps/readur/uploads:/app/uploads
      - /mnt/pool/apps/readur/watch:/app/watch
```

## Network Filesystem Mounts

### NFS Mounts

```bash
# Mount NFS share
sudo mount -t nfs 192.168.1.100:/documents /mnt/nfs/documents

# Add to docker-compose.yml
volumes:
  - /mnt/nfs/documents:/app/watch
environment:
  - WATCH_INTERVAL_SECONDS=60
  - FILE_STABILITY_CHECK_MS=1000
  - FORCE_POLLING_WATCH=1
```

### SMB/CIFS Mounts

```bash
# Mount SMB share
sudo mount -t cifs //server/share /mnt/smb/shared -o username=user,password=pass

# Docker volume configuration
volumes:
  - /mnt/smb/shared:/app/watch
environment:
  - WATCH_INTERVAL_SECONDS=30
  - FILE_STABILITY_CHECK_MS=2000
```

### S3 Mounts

```bash
# Mount S3 bucket using s3fs
s3fs mybucket /mnt/s3/bucket -o passwd_file=~/.passwd-s3fs

# Docker configuration for S3
volumes:
  - /mnt/s3/bucket:/app/watch
environment:
  - WATCH_INTERVAL_SECONDS=120
  - FILE_STABILITY_CHECK_MS=5000
  - FORCE_POLLING_WATCH=1
```

## SSL/HTTPS Setup

### Nginx Configuration

```nginx
server {
    listen 443 ssl http2;
    server_name readur.yourdomain.com;
    
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    
    location / {
        proxy_pass http://localhost:8000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # For file uploads
        client_max_body_size 100M;
        proxy_read_timeout 300s;
        proxy_send_timeout 300s;
    }
}
```

### Traefik Configuration

```yaml
services:
  readur:
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.readur.rule=Host(`readur.yourdomain.com`)"
      - "traefik.http.routers.readur.tls=true"
      - "traefik.http.routers.readur.tls.certresolver=letsencrypt"
```

> 📘 **For more reverse proxy configurations** including Apache, Caddy, custom ports, load balancing, and advanced scenarios, see [REVERSE_PROXY.md](./REVERSE_PROXY.md).

## Health Checks

Add health checks to your Docker configuration. The Readur Docker image includes `curl` for health checking.

**Important:** The port in the healthcheck URL must match your `SERVER_PORT` or the port specified in `SERVER_ADDRESS`:

```yaml
services:
  readur:
    environment:
      # If using SERVER_ADDRESS
      - SERVER_ADDRESS=0.0.0.0:8000  # Port 8000
      # Or if using SERVER_PORT
      # - SERVER_PORT=8000            # Port 8000
    
    healthcheck:
      # Port in URL must match the SERVER_PORT/SERVER_ADDRESS port above
      test: ["CMD", "curl", "-f", "http://localhost:8000/api/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
```

For example, if you change the server to run on port 3000:
- Set `SERVER_PORT=3000` or `SERVER_ADDRESS=0.0.0.0:3000`
- Update healthcheck to: `http://localhost:3000/api/health`

## Backup Strategy

> **Upgrading PostgreSQL?** See the dedicated [PostgreSQL Upgrade Guide](postgres-upgrade.md) for major version migrations (e.g., 15 → 16).

Create an automated backup script:

```bash
#!/bin/bash
# backup.sh - Automated backup script

BACKUP_DIR="/path/to/backups"
DATE=$(date +%Y%m%d_%H%M%S)

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Backup database
docker exec readur-postgres-1 pg_dump -U readur readur | gzip > "$BACKUP_DIR/db_backup_$DATE.sql.gz"

# Backup uploaded files
tar -czf "$BACKUP_DIR/uploads_backup_$DATE.tar.gz" -C ./data uploads/

# Clean old backups (keep 30 days)
find "$BACKUP_DIR" -name "db_backup_*.sql.gz" -mtime +30 -delete
find "$BACKUP_DIR" -name "uploads_backup_*.tar.gz" -mtime +30 -delete

echo "Backup completed: $DATE"
```

Add to crontab for daily backups:
```bash
0 2 * * * /path/to/backup.sh >> /var/log/readur-backup.log 2>&1
```

### Restore from Backup

```bash
# Restore database
gunzip -c db_backup_20240101_020000.sql.gz | docker exec -i readur-postgres-1 psql -U readur readur

# Restore files
tar -xzf uploads_backup_20240101_020000.tar.gz -C ./data
```

## Monitoring

Monitor your deployment with Docker stats:

```bash
# Real-time resource usage
docker stats

# Container logs
docker compose logs -f readur

# Watch folder activity
docker compose logs -f readur | grep watcher

# PostgreSQL query performance
docker exec readur-postgres-1 psql -U readur -c "SELECT * FROM pg_stat_statements ORDER BY total_time DESC LIMIT 10;"
```

### Prometheus Metrics

Readur exposes metrics at `/metrics` endpoint:

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'readur'
    static_configs:
      - targets: ['readur:8000']
```

## Deployment Platforms

### Docker Swarm

**Note:** Readur is a single-instance application. If using Docker Swarm, ensure replicas is set to 1.

```yaml
version: '3.8'
services:
  readur:
    image: ghcr.io/readur/readur:main
    deploy:
      replicas: 1  # MUST be 1 - Readur doesn't support multiple instances
      restart_policy:
        condition: on-failure
      placement:
        constraints: [node.role == worker]
    networks:
      - readur-network
    secrets:
      - jwt_secret
      - db_password

secrets:
  jwt_secret:
    external: true
  db_password:
    external: true
```

### Kubernetes

**Important:** Readur is a single-instance application. Always set replicas to 1.

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: readur
spec:
  replicas: 1  # MUST be 1 - Readur doesn't support multiple instances
  selector:
    matchLabels:
      app: readur
  template:
    spec:
      containers:
      - name: readur
        image: ghcr.io/readur/readur:main
        env:
        - name: JWT_SECRET
          valueFrom:
            secretKeyRef:
              name: readur-secrets
              key: jwt-secret
        resources:
          limits:
            memory: "4Gi"  # Increase for single instance
            cpu: "4"        # Increase for single instance
          requests:
            memory: "1Gi"
            cpu: "1"
```

### Cloud Platforms

- **AWS**: Use ECS with RDS PostgreSQL
- **Google Cloud**: Deploy to Cloud Run with Cloud SQL
- **Azure**: Use Container Instances with Azure Database
- **DigitalOcean**: App Platform with Managed Database

## Security Considerations

### Production Checklist

- [ ] **CRITICAL: Change JWT_SECRET from default value**
- [ ] Change default admin password
- [ ] Generate strong JWT secret (use `openssl rand -base64 32`)
- [ ] Use HTTPS/SSL in production
- [ ] **Never disable SSL verification in production** (S3_VERIFY_SSL must be true)
- [ ] Restrict database network access
- [ ] Set proper file permissions
- [ ] Enable firewall rules
- [ ] Regular security updates
- [ ] Monitor access logs
- [ ] Implement rate limiting
- [ ] Enable audit logging
- [ ] **Never expose secrets in command lines** (use env vars or config files)

### Recommended Production Setup

```bash
# Generate secure secrets - ALWAYS DO THIS!
JWT_SECRET=$(openssl rand -base64 64)  # NEVER use default values
DB_PASSWORD=$(openssl rand -base64 32)

# WARNING: Default JWT_SECRET values are insecure
# Always generate new secrets for production

# Restrict file permissions
chmod 600 .env
chmod 700 ./data/uploads

# Use read-only root filesystem
docker run --read-only --tmpfs /tmp ...
```

## Next Steps

- Configure [monitoring and alerting](health-monitoring-guide.md)
- Review [security best practices](security-guide.md)
- Set up [automated backups](#backup-strategy)
- Explore [database guardrails](dev/DATABASE_GUARDRAILS.md)