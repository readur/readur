# Self-Hosting Guide

## Overview

This comprehensive guide covers everything you need to successfully self-host Readur in your own infrastructure. Whether you're deploying for personal use, your organization, or offering it as a service, this guide provides complete instructions for setup, configuration, and maintenance.

## Self-Hosting Options

### Docker Deployment (Recommended)

The simplest and most reliable deployment method:

- **Single-server setup**: All components on one machine (standard deployment)
- **Docker Compose**: Manage containers easily
- **Note**: Readur is a single-instance application and does not support clustering or multiple server instances

[Quick Start with Docker →](../quickstart/docker.md)

### Bare Metal Installation

Direct installation on Linux servers:

- **System packages**: Install from distribution repositories
- **From source**: Build and install manually
- **SystemD services**: Managed by system init

[Bare Metal Installation →](./bare-metal.md)

### Cloud Platform Deployment

Deploy on managed cloud services:

- **AWS**: EC2, RDS, S3, and ECS
- **Google Cloud**: Compute Engine, Cloud SQL, GCS
- **Azure**: Virtual Machines, Database, Blob Storage
- **DigitalOcean**: Droplets, Managed Database, Spaces

[Cloud Deployment Guide →](./cloud-deployment.md)

## System Requirements

### Minimum Requirements

For personal use or small teams (1-10 users):

- **CPU**: 2 cores (x86_64 or ARM64)
- **RAM**: 4GB
- **Storage**: 20GB for system + document storage
- **OS**: Linux (Ubuntu 20.04+, Debian 11+, RHEL 8+)
- **Docker**: Version 20.10+ (if using containers)

### Recommended Production Setup

For organizations (10-100 users):

- **CPU**: 4-8 cores
- **RAM**: 16GB
- **Storage**: 100GB SSD for system, 1TB+ for documents
- **Database**: PostgreSQL 14+ with 50GB storage
- **Cache**: Redis 6+ with 2GB RAM
- **Network**: 100Mbps+ connection

### Enterprise Scale

For large deployments (100+ users):

- **Server**: Single high-performance server with ample resources
- **CPU**: 8-16 cores for concurrent processing
- **RAM**: 32GB+ for large document processing
- **Database**: PostgreSQL with performance tuning
- **Storage**: S3-compatible object storage for scalability
- **Note**: Vertical scaling only - add more resources to the single server

## Installation Methods

### Method 1: Docker Compose (Simplest)

Perfect for single-server deployments:

```bash
# Download and configure
git clone https://github.com/readur/readur.git
cd readur
cp .env.example .env
nano .env  # Configure settings

# Start services
docker-compose up -d

# Verify installation
docker-compose ps
curl http://localhost:8000/health
```

[Detailed Docker Guide →](./docker-setup.md)

### Method 2: Kubernetes (Container Orchestration)

For production deployments with container orchestration:

```yaml
# readur-deployment.yaml
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
    metadata:
      labels:
        app: readur
    spec:
      containers:
      - name: readur
        image: readur/readur:latest
        ports:
        - containerPort: 8000
```

[Kubernetes Deployment →](./kubernetes.md)

### Method 3: Ansible Automation

Automated deployment across multiple servers:

```yaml
# playbook.yml
- hosts: readur_servers
  roles:
    - postgresql
    - redis
    - readur
  vars:
    readur_version: "2.5.4"
    postgres_version: "14"
```

[Ansible Playbook →](./ansible.md)

## Configuration

### Essential Configuration

These settings must be configured before first run:

```bash
# Security
APP_SECRET_KEY=<generate-with-openssl-rand-hex-32>
ADMIN_PASSWORD=<strong-password>

# Database
DATABASE_URL=postgresql://user:pass@localhost/readur
POSTGRES_PASSWORD=<secure-password>

# Storage
STORAGE_BACKEND=s3  # or 'local'
S3_BUCKET=readur-documents
S3_ACCESS_KEY_ID=<your-key>
S3_SECRET_ACCESS_KEY=<your-secret>
```

[Complete Configuration Reference →](./configuration.md)

### Storage Configuration

Choose and configure your storage backend:

#### Local Storage

```bash
STORAGE_BACKEND=local
LOCAL_STORAGE_PATH=/data/readur/documents
```

#### S3-Compatible Storage

```bash
STORAGE_BACKEND=s3
S3_ENDPOINT=https://s3.amazonaws.com
S3_BUCKET=my-readur-bucket
S3_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
S3_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
```

[Storage Setup Guide →](./storage.md)

### Authentication Setup

Configure authentication methods:

#### Local Authentication

```bash
AUTH_METHOD=local
ENABLE_REGISTRATION=false
REQUIRE_EMAIL_VERIFICATION=true
```

#### OIDC/SSO Integration

```bash
AUTH_METHOD=oidc
OIDC_ISSUER=https://auth.company.com
OIDC_CLIENT_ID=readur
OIDC_CLIENT_SECRET=<secret>
```

[Authentication Guide →](./authentication.md)

## Network Configuration

### Reverse Proxy Setup

Configure NGINX for HTTPS:

```nginx
server {
    listen 443 ssl http2;
    server_name readur.company.com;
    
    ssl_certificate /etc/ssl/certs/readur.crt;
    ssl_certificate_key /etc/ssl/private/readur.key;
    
    location / {
        proxy_pass http://localhost:8000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

[Reverse Proxy Guide →](./reverse-proxy.md)

### Firewall Configuration

Essential firewall rules:

```bash
# Allow HTTPS
sudo ufw allow 443/tcp

# Allow SSH (restrict to your IP)
sudo ufw allow from YOUR_IP to any port 22

# Enable firewall
sudo ufw enable
```

## Backup and Recovery

### Automated Backups

Set up daily backups with retention:

```bash
#!/bin/bash
# backup.sh

# Backup database
pg_dump $DATABASE_URL > backup-$(date +%Y%m%d).sql

# Backup documents (if local storage)
tar -czf documents-$(date +%Y%m%d).tar.gz /data/readur/documents

# Upload to S3
aws s3 cp backup-*.sql s3://backups/readur/
aws s3 cp documents-*.tar.gz s3://backups/readur/

# Clean old backups (keep 30 days)
find . -name "backup-*.sql" -mtime +30 -delete
```

[Backup Strategy Guide →](./backup.md)

### Disaster Recovery

Restore from backup:

```bash
# Restore database
PGPASSWORD="${DB_PASSWORD}" psql -h localhost -U readur -d readur < backup-20240315.sql

# Restore documents
tar -xzf documents-20240315.tar.gz -C /

# Verify integrity by checking document count
docker-compose exec readur psql -U readur -d readur -c "SELECT COUNT(*) FROM documents;"
```

## Security Hardening

### Security Checklist

- [ ] Change all default passwords
- [ ] Enable HTTPS with valid certificates
- [ ] Configure firewall rules
- [ ] Disable unnecessary services
- [ ] Enable audit logging
- [ ] Set up intrusion detection
- [ ] Regular security updates
- [ ] Implement rate limiting

[Security Best Practices →](./security.md)

### SSL/TLS Configuration

Obtain and configure SSL certificates:

```bash
# Using Let's Encrypt
sudo certbot certonly --webroot -w /var/www/html -d readur.company.com

# Auto-renewal
sudo certbot renew --dry-run
```

## Monitoring and Maintenance

### Health Monitoring

Set up monitoring endpoints:

```bash
# Health check
curl http://localhost:8000/health

# Metrics
curl http://localhost:8000/metrics

# Status page
curl http://localhost:8000/status
```

[Monitoring Setup →](./monitoring.md)

### Performance Tuning

Optimize for your workload:

```bash
# OCR processing
OCR_WORKERS=4
OCR_MAX_PARALLEL=8
OCR_QUEUE_SIZE=100

# Database connections
POSTGRES_MAX_CONNECTIONS=200
DATABASE_POOL_SIZE=20

# Caching
REDIS_MAX_MEMORY=4gb
CACHE_TTL=3600
```

[Performance Guide →](./performance.md)

### Updates and Upgrades

Keep your installation current:

```bash
# Backup first
./backup.sh

# Pull latest version
docker-compose pull

# Run migrations
docker-compose exec readur alembic upgrade head

# Restart services
docker-compose down && docker-compose up -d
```

[Update Procedures →](./updates.md)

## Troubleshooting

### Common Issues

#### Service Won't Start

```bash
# Check logs
docker-compose logs readur
journalctl -u readur

# Verify permissions
ls -la /data/readur/
```

#### OCR Not Processing

```bash
# Check worker status
docker-compose logs ocr-worker

# Monitor queue
redis-cli llen ocr_queue
```

#### Database Connection Failed

```bash
# Test connection
psql $DATABASE_URL -c "SELECT 1"

# Check firewall
telnet postgres_host 5432
```

[Complete Troubleshooting Guide →](./troubleshooting.md)

## Migration from Other Systems

### Migrating from Paperless-ngx

```bash
# Export from Paperless
python manage.py document_exporter ../export

# Import to Readur
python import_paperless.py ../export --preserve-metadata
```

### Migrating from Mayan EDMS

```bash
# Use migration tool
python migrate_mayan.py \
  --source-db postgresql://mayan_db \
  --target-db postgresql://readur_db
```

[Migration Guide →](./migration.md)

## Support and Resources

### Getting Help

- **Documentation**: You're here!
- **GitHub Issues**: [Report bugs](https://github.com/readur/readur/issues)
- **Discussions**: [Community forum](https://github.com/readur/readur/discussions)
- **Chat**: [Discord server](https://discord.gg/readur)

### Useful Commands

```bash
# View logs
docker-compose logs -f

# Access shell
docker-compose exec readur bash

# Database console
docker-compose exec postgres psql -U readur

# Redis CLI
docker-compose exec redis redis-cli

# Run management command
# For Rust CLI tools:
docker-compose exec readur /app/migrate_to_s3 --help
# Or during development:
docker-compose exec readur cargo run --bin migrate_to_s3 -- --help
```

## Next Steps

1. [Complete installation](../quickstart/self-hosting.md)
2. [Configure storage backend](./storage.md)
3. [Set up authentication](./authentication.md)
4. [Enable HTTPS](./reverse-proxy.md)
5. [Configure backups](./backup.md)
6. [Set up monitoring](./monitoring.md)