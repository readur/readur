# Deployment Guide

This guide covers production deployment strategies, SSL setup, monitoring, backups, and best practices for running Readur in production.

## Table of Contents

- [Production Docker Compose](#production-docker-compose)
- [Network Filesystem Mounts](#network-filesystem-mounts)
  - [NFS Mounts](#nfs-mounts)
  - [SMB/CIFS Mounts](#smbcifs-mounts)
  - [S3 Mounts](#s3-mounts)
- [SSL/HTTPS Setup](#sslhttps-setup)
  - [Nginx Configuration](#nginx-configuration)
  - [Traefik Configuration](#traefik-configuration)
- [Health Checks](#health-checks)
- [Backup Strategy](#backup-strategy)
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
    image: readur:latest
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
    image: postgres:15
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

Add health checks to your Docker configuration:

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

## Backup Strategy

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

```yaml
version: '3.8'
services:
  readur:
    image: readur:latest
    deploy:
      replicas: 2
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

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: readur
spec:
  replicas: 3
  selector:
    matchLabels:
      app: readur
  template:
    spec:
      containers:
      - name: readur
        image: readur:latest
        env:
        - name: JWT_SECRET
          valueFrom:
            secretKeyRef:
              name: readur-secrets
              key: jwt-secret
        resources:
          limits:
            memory: "2Gi"
            cpu: "2"
          requests:
            memory: "512Mi"
            cpu: "500m"
```

### Cloud Platforms

- **AWS**: Use ECS with RDS PostgreSQL
- **Google Cloud**: Deploy to Cloud Run with Cloud SQL
- **Azure**: Use Container Instances with Azure Database
- **DigitalOcean**: App Platform with Managed Database

## Security Considerations

### Production Checklist

- [ ] Change default admin password
- [ ] Generate strong JWT secret
- [ ] Use HTTPS/SSL in production
- [ ] Restrict database network access
- [ ] Set proper file permissions
- [ ] Enable firewall rules
- [ ] Regular security updates
- [ ] Monitor access logs
- [ ] Implement rate limiting
- [ ] Enable audit logging

### Recommended Production Setup

```bash
# Generate secure secrets
JWT_SECRET=$(openssl rand -base64 64)
DB_PASSWORD=$(openssl rand -base64 32)

# Restrict file permissions
chmod 600 .env
chmod 700 ./data/uploads

# Use read-only root filesystem
docker run --read-only --tmpfs /tmp ...
```

## Next Steps

- Configure [monitoring and alerting](monitoring-usage)
- Review [security best practices](security)
- Set up [automated backups](#backup-strategy)
- Explore [database guardrails](dev/DATABASE_GUARDRAILS.md)