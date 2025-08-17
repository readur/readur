# Self-Hosting Quick Start

## Purpose

Get Readur running on your own server in 15 minutes. This guide covers the essential steps for a basic self-hosted deployment.

## Prerequisites

- Linux server (Ubuntu 20.04+ or similar)
- Docker and Docker Compose installed
- 4GB RAM minimum (8GB recommended)
- 20GB free disk space for documents
- Domain name (optional, for HTTPS access)

## Step 1: Download Readur

Clone the repository and navigate to the directory:

```bash
git clone https://github.com/readur/readur.git
cd readur
```

## Step 2: Configure Environment

Create your environment file from the template:

```bash
cp .env.example .env
```

Edit the `.env` file with essential settings:

```bash
# Core settings
APP_SECRET_KEY=your-secret-key-here  # Generate with: openssl rand -hex 32
POSTGRES_PASSWORD=secure-password-here
ADMIN_PASSWORD=your-admin-password

# Storage (choose one)
STORAGE_BACKEND=local  # or 's3' for cloud storage
LOCAL_STORAGE_PATH=/data/readur/documents

# Optional: External access
APP_HOST=0.0.0.0  # Allow external connections
APP_PORT=8000
```

## Step 3: Start Services

Launch Readur with Docker Compose:

```bash
docker-compose up -d
```

Monitor the startup process:

```bash
docker-compose logs -f
```

Wait for the message: `Application startup complete`

## Step 4: Access Readur

Open your browser and navigate to:
- **Local access**: http://localhost:8000
- **Network access**: http://your-server-ip:8000

Login with default credentials:
- Username: `admin`
- Password: (the one you set in ADMIN_PASSWORD)

## Step 5: Initial Setup

### Upload Your First Document

1. Click **Upload** in the top navigation
2. Select PDF or image files
3. Enable **Process with OCR** for scanned documents
4. Click **Upload Files**

### Configure OCR Languages

1. Go to **Settings** → **OCR Configuration**
2. Select your primary languages
3. Adjust OCR workers based on server capacity

### Set Up Automatic Import (Optional)

1. Navigate to **Settings** → **Sources**
2. Add a watch folder:
   ```
   Path: /data/watch
   Auto-process: Enabled
   OCR: Enabled
   ```
3. Any files placed in this folder will be automatically imported

## Next Steps

### Security Hardening

For production use, implement these security measures:

1. **Enable HTTPS**: See [Reverse Proxy Setup](../self-hosting/reverse-proxy.md)
2. **Change default passwords**: Update all default credentials
3. **Configure firewall**: Restrict access to necessary ports only
4. **Enable authentication**: Set up [OIDC/SSO](../self-hosting/authentication.md)

### Performance Optimization

Tune for your workload:

```bash
# In .env file
OCR_WORKERS=4  # Increase for faster processing
POSTGRES_MAX_CONNECTIONS=100
UPLOAD_MAX_SIZE_MB=100
```

### Backup Configuration

Set up automated backups:

```bash
# Create backup script
cat > backup.sh << 'EOF'
#!/bin/bash
docker-compose exec postgres pg_dump -U readur > backup-$(date +%Y%m%d).sql
tar -czf documents-$(date +%Y%m%d).tar.gz /data/readur/documents
EOF

chmod +x backup.sh
```

## Troubleshooting

### Container Won't Start

Check logs for specific errors:
```bash
docker-compose logs readur
docker-compose logs postgres
```

### OCR Not Processing

Verify OCR service is running:
```bash
docker-compose ps
docker-compose logs ocr-worker
```

### Can't Access from Browser

Check firewall settings:
```bash
sudo ufw allow 8000/tcp
```

Verify service is listening:
```bash
netstat -tlnp | grep 8000
```

## Related Documentation

- [Complete Self-Hosting Guide](../self-hosting/index.md) - Comprehensive setup and configuration
- [Storage Configuration](../self-hosting/storage.md) - S3, WebDAV, and local storage options
- [Performance Tuning](../self-hosting/performance.md) - Optimize for your hardware
- [Backup Strategies](../self-hosting/backup.md) - Protect your data