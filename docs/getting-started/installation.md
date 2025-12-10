# Installation Guide

Deploy Readur document management system with OCR capabilities using Docker.

## Prerequisites

### System Requirements

#### Minimum Requirements
- **CPU**: 2 cores (x86_64 or ARM64)
- **RAM**: 4GB (system) + 1GB per concurrent OCR job
- **Storage**: 10GB for application + space for documents
- **OS**: Linux, macOS, or Windows with Docker support

#### Recommended for Production
- **CPU**: 4+ cores for parallel OCR processing
- **RAM**: 8GB minimum, 16GB for heavy workloads
- **Storage**: SSD for database, adequate space for document growth
- **Network**: Stable connection for source synchronization

### Software Requirements

```bash
# Check Docker version (20.10+ required)
docker --version

# Check Docker Compose version (2.0+ required)
docker-compose --version

# Verify Docker is running
docker ps
```

## Installation Methods

### Quick Start with Docker Compose (Recommended)

#### 1. Clone the Repository

```bash
# Clone the repository
git clone https://github.com/readur/readur.git
cd readur

# Review the configuration
cat docker-compose.yml
```

#### 2. Configure Environment

Create a `.env` file with your settings:

```bash
# Security - CHANGE THESE!
JWT_SECRET=$(openssl rand -base64 32)
DB_PASSWORD=$(openssl rand -base64 32)
ADMIN_PASSWORD=your_secure_password_here

# OCR Configuration
OCR_LANGUAGE=eng  # or: deu, fra, spa, etc.
CONCURRENT_OCR_JOBS=2

# Storage Paths (create these directories)
UPLOAD_PATH=./data/uploads
WATCH_FOLDER=./data/watch

# Optional: S3 Storage (instead of local)
# STORAGE_BACKEND=s3
# S3_BUCKET=readur-documents
# S3_REGION=us-east-1
# AWS_ACCESS_KEY_ID=your_key
# AWS_SECRET_ACCESS_KEY=your_secret
```

#### 3. Create Required Directories

```bash
# Create data directories
mkdir -p data/{uploads,watch,postgres}

# Set appropriate permissions
chmod 755 data/uploads data/watch
```

#### 4. Start the Application

```bash
# Start all services
docker-compose up -d

# Monitor startup logs
docker-compose logs -f

# Wait for "Server started on 0.0.0.0:8000"
```

#### 5. Verify Installation

```bash
# Check service health
docker-compose ps

# Test the API endpoint
curl http://localhost:8000/health

# Expected response:
# {"status":"healthy","database":"connected","ocr":"ready"}
```

### Production Deployment with Custom Configuration

#### 1. Create Production Compose File

Create `docker-compose.prod.yml`:

```yaml
services:
  readur:
    image: ghcr.io/readur/readur:main
    ports:
      - "8000:8000"
    environment:
      - DATABASE_URL=postgresql://readur:${DB_PASSWORD}@postgres:5432/readur
      - JWT_SECRET=${JWT_SECRET}
      - SERVER_ADDRESS=0.0.0.0:8000
      - UPLOAD_PATH=/app/uploads
      - CONCURRENT_OCR_JOBS=4
      - MAX_FILE_SIZE_MB=100
    volumes:
      - ./data/uploads:/app/uploads
      - /mnt/shared/documents:/app/watch:ro
    depends_on:
      postgres:
        condition: service_healthy
    restart: unless-stopped
    deploy:
      resources:
        limits:
          memory: 2G
          cpus: '2.0'

  postgres:
    image: postgres:15-alpine
    environment:
      - POSTGRES_USER=readur
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=readur
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U readur"]
      interval: 10s
      timeout: 5s
      retries: 5
    restart: unless-stopped

volumes:
  postgres_data:
```

#### 2. Deploy with Production Settings

```bash
# Use production configuration
docker-compose -f docker-compose.prod.yml up -d

# Enable automatic startup
sudo systemctl enable docker
```

### Kubernetes Deployment

#### Using Helm Chart

```bash
# Add Readur Helm repository
helm repo add readur https://charts.readur.app
helm repo update

# Install with custom values
helm install readur readur/readur \
  --set image.tag=latest \
  --set postgresql.auth.password=$DB_PASSWORD \
  --set auth.jwtSecret=$JWT_SECRET \
  --set persistence.size=50Gi \
  --set ingress.enabled=true \
  --set ingress.hostname=readur.example.com
```

#### Using Raw Manifests

```bash
# Apply Kubernetes manifests
kubectl create namespace readur
kubectl apply -f https://raw.githubusercontent.com/readur/readur/main/k8s/

# Check deployment status
kubectl -n readur get pods
kubectl -n readur get svc
```

### Docker Run (Development Only)

For quick testing without persistence:

```bash
# Run with in-memory database (data lost on restart)
docker run -d \
  --name readur \
  -p 8000:8000 \
  -e DATABASE_URL=sqlite:///tmp/readur.db \
  -e JWT_SECRET=dev-only-secret \
  readur:latest

# Access logs
docker logs -f readur
```

## Post-Installation Setup

### Initial Login

1. **Access the Web Interface**
   ```
   http://localhost:8000
   ```

2. **Login with Admin Credentials**
   - Username: `admin`
   - Password: Check the container logs for your auto-generated password

   On first startup, Readur generates a secure admin password and displays it in the logs.
   View the logs with `docker compose logs readur` and look for "READUR ADMIN USER CREATED".

   **Save this password immediately - it won't be shown again.**

3. **Resetting Admin Password**
   If you lose your password, reset it with:
   ```bash
   docker exec readur readur reset-admin-password
   ```

### Essential Configuration

#### 1. Configure OCR Languages

```bash
# Check available languages
docker exec readur tesseract --list-langs

# Add additional language packs if needed
docker exec readur apt-get update
docker exec readur apt-get install -y tesseract-ocr-deu  # German
docker exec readur apt-get install -y tesseract-ocr-fra  # French
docker exec readur apt-get install -y tesseract-ocr-spa  # Spanish
```

#### 2. Set Up Document Sources

1. Navigate to Settings → Sources
2. Add your document sources:
   - **Local Folders**: Mount volumes in docker-compose.yml
   - **WebDAV**: Configure Nextcloud/ownCloud connections
   - **S3 Buckets**: Add AWS S3 or compatible storage

#### 3. Configure User Authentication

**For Local Users:**
- Settings → User Management → Create User
- Assign appropriate roles (User or Admin)

**For SSO/OIDC:**
```bash
# Add to your .env file
OIDC_ENABLED=true
OIDC_ISSUER=https://auth.example.com
OIDC_CLIENT_ID=readur-client
OIDC_CLIENT_SECRET=your-secret
```

#### 4. Adjust Performance Settings

```bash
# Edit .env for your workload
CONCURRENT_OCR_JOBS=4        # Increase for faster processing
OCR_TIMEOUT_SECONDS=300      # Increase for large documents
MAX_FILE_SIZE_MB=100         # Adjust based on your documents
MEMORY_LIMIT_MB=2048         # Increase for better performance
```

## Verification & Health Checks

### Service Health

```bash
# Check all services are running
docker-compose ps

# Expected output:
NAME                STATUS              PORTS
readur              running (healthy)   0.0.0.0:8000->8000/tcp
postgres            running (healthy)   5432/tcp
```

### API Health Check

```bash
# Test the health endpoint
curl -s http://localhost:8000/health | jq

# Expected response:
{
  "status": "healthy",
  "version": "2.5.4",
  "database": "connected",
  "ocr_service": "ready",
  "storage": "available",
  "queue_size": 0
}
```

### Database Connectivity

```bash
# Test database connection
docker exec readur-postgres psql -U readur -c "SELECT version();"

# Check tables were created
docker exec readur-postgres psql -U readur -d readur -c "\dt"
```

### OCR Functionality

```bash
# Test OCR engine
docker exec readur tesseract --version

# Upload a test document
curl -X POST http://localhost:8000/api/upload \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -F "file=@test.pdf"
```

## Troubleshooting Installation

### Common Issues and Solutions

#### Port Already in Use

```bash
# Check what's using port 8000
sudo lsof -i :8000

# Solution 1: Stop the conflicting service
sudo systemctl stop conflicting-service

# Solution 2: Use a different port
# Edit docker-compose.yml:
ports:
  - "8080:8000"  # Change 8080 to your preferred port
```

#### Database Connection Failed

```bash
# Check PostgreSQL logs
docker-compose logs postgres

# Common fixes:
# 1. Ensure PostgreSQL is fully started
docker-compose restart postgres
sleep 10
docker-compose restart readur

# 2. Reset database (WARNING: Deletes all data)
docker-compose down -v
docker-compose up -d
```

#### OCR Processing Stuck

```bash
# Check OCR queue status
curl http://localhost:8000/api/admin/queue/status

# Restart OCR workers
docker-compose restart readur

# Increase timeout for large files
# Add to .env:
OCR_TIMEOUT_SECONDS=600
```

#### Docker Permission Denied

```bash
# Linux: Add user to docker group
sudo usermod -aG docker $USER
newgrp docker

# Verify docker access
docker ps
```

#### Insufficient Memory

```bash
# Check container memory usage
docker stats readur

# Increase memory limits in docker-compose.yml:
deploy:
  resources:
    limits:
      memory: 4G  # Increase as needed
```

### Getting Help

1. **Check Logs**
   ```bash
   # Application logs
   docker-compose logs -f readur
   
   # Database logs
   docker-compose logs -f postgres
   ```

2. **Enable Debug Mode**
   ```bash
   # Add to .env
   LOG_LEVEL=DEBUG
   
   # Restart services
   docker-compose restart
   ```

3. **Community Support**
   - [GitHub Issues](https://github.com/readur/readur/issues)
   - [Documentation](https://docs.readur.app)
   - [Discord Community](https://discord.gg/readur)

## Next Steps

### Essential Reading

1. **[User Guide](../user-guide.md)**
   - Upload and manage documents
   - Configure OCR processing
   - Master search features
   - Organize with labels

2. **[Configuration Reference](../configuration-reference.md)**
   - Complete environment variable list
   - Performance tuning
   - Storage configuration
   - Security settings

3. **[Deployment Guide](../deployment.md)**
   - SSL/TLS setup with reverse proxy
   - Backup and restore procedures
   - Monitoring and alerts
   - Scaling strategies

### Advanced Setup

4. **[Sources Guide](../sources-guide.md)**
   - WebDAV integration
   - S3 bucket synchronization
   - Watch folder configuration
   - Automated imports

5. **[OIDC Setup](../oidc-setup.md)**
   - Enterprise SSO integration
   - Azure AD configuration
   - Google Workspace setup
   - Keycloak integration

6. **[API Reference](../api-reference.md)**
   - REST API endpoints
   - Authentication
   - Automation examples
   - Webhook integration

### Quick Test

Upload your first document:

```bash
# 1. Login to get token (use your generated password from the logs)
TOKEN=$(curl -s -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"YOUR_GENERATED_PASSWORD"}' | jq -r .token)

# 2. Upload a PDF
curl -X POST http://localhost:8000/api/documents/upload \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@sample.pdf"

# 3. Check OCR status
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8000/api/documents
```