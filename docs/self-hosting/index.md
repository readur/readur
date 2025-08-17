# Self-Hosting Guide

This guide walks you through everything you need to know to run Readur on your own infrastructure. Whether you're setting up a personal document system, deploying for your team, or running Readur for an entire organization, you'll find practical guidance for installation, configuration, and ongoing maintenance.

## Choosing Your Deployment Method

### Docker Deployment (Recommended)

Docker provides the simplest and most reliable way to deploy Readur. All components run on a single server using Docker Compose to manage the containers. This approach works well because Readur is designed as a single-instance application - you don't need to worry about clustering or load balancing across multiple servers. The containerized setup ensures consistency and makes updates straightforward.

[Get started with Docker →](../quickstart/docker.md)

### Bare Metal Installation

For organizations that prefer traditional server installations, you can install Readur directly on Linux servers. This method gives you complete control over the system packages and lets you integrate with existing infrastructure management tools. You'll install Readur from distribution repositories or build from source, then manage it with SystemD services like other server applications.

[Bare metal installation guide →](../deployment.md)

### Cloud Platform Deployment

Major cloud providers offer managed services that can significantly simplify your Readur deployment. Use EC2, RDS, and S3 on AWS, or Compute Engine, Cloud SQL, and Cloud Storage on Google Cloud. Azure provides Virtual Machines, managed databases, and Blob Storage. DigitalOcean offers Droplets, managed databases, and Spaces storage. These managed services handle infrastructure maintenance while you focus on managing your documents.

[Cloud deployment guide →](../deployment.md)

## System Requirements

### For Personal Use and Small Teams

If you're setting up Readur for personal use or a small team of 1-10 users, you can start with modest hardware. A 2-core CPU (either x86_64 or ARM64), 4GB of RAM, and 20GB of storage will handle basic document processing. Add more storage based on how many documents you plan to manage. Any modern Linux distribution works - Ubuntu 20.04+, Debian 11+, or RHEL 8+ are well-tested options. If you're using Docker, make sure you have version 20.10 or newer.

### Production Deployments for Organizations

Organizations with 10-100 users should plan for more substantial hardware to handle concurrent document processing and user activity. A 4-8 core CPU and 16GB of RAM provide comfortable performance headroom. Use SSD storage for the system and database (100GB minimum), with additional storage for documents based on your collection size - 1TB is a reasonable starting point for most organizations.

Your PostgreSQL database should have at least 50GB of dedicated storage, and Redis needs about 2GB of RAM for caching and queue management. A reliable network connection (100Mbps or better) ensures responsive performance when multiple users are working simultaneously.

### Enterprise Scale Deployments

Large deployments serving 100+ users require a single high-performance server with substantial resources. Plan for 8-16 CPU cores to handle concurrent OCR processing efficiently, and 32GB or more RAM for processing large batches of documents without performance degradation.

For storage, consider S3-compatible object storage for unlimited scalability and better backup options. Remember that Readur scales vertically rather than horizontally - you add more resources to a single powerful server rather than distributing load across multiple machines.

## Installation Methods

### Docker Compose (Recommended for Most Users)

Docker Compose provides the simplest path to a working Readur installation. This method handles all the service dependencies automatically and works well for single-server deployments:

```bash
# Clone the repository and configure
git clone https://github.com/readur/readur.git
cd readur
cp .env.example .env
nano .env  # Edit configuration settings

# Start all services
docker-compose up -d

# Verify everything is working
docker-compose ps
curl http://localhost:8000/health
```

This approach downloads pre-built containers, sets up the database, configures networking between services, and starts everything with a single command.

[Follow the detailed Docker guide →](../quickstart/docker.md)

### Kubernetes for Organizations with Container Orchestration

If your organization already uses Kubernetes for container management, you can deploy Readur within your existing cluster. This approach provides consistent container lifecycle management and integrates with your existing monitoring and backup systems:

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

This Kubernetes configuration creates a single instance deployment (note the `replicas: 1` setting - Readur is designed as a single-instance application and doesn't support horizontal scaling). You'll also need to configure services, persistent volumes for document storage, and secrets for database credentials.

[Complete Kubernetes deployment guide →](../deployment.md#kubernetes)

### Ansible for Automated Infrastructure

Organizations managing multiple servers or standardized deployments benefit from Ansible automation. This approach lets you deploy Readur consistently across different environments:

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

This playbook installs PostgreSQL, Redis, and Readur with specified versions across all servers in your `readur_servers` inventory group. You can customize variables for different environments (development, staging, production) while maintaining consistency in the deployment process.

[Complete Ansible playbook →](../deployment.md)

## Configuration

### Critical Settings You Must Configure

Before starting Readur for the first time, you need to configure several essential settings for security and functionality. These settings control how Readur encrypts session data, connects to the database, and stores documents:

```bash
# Security settings - generate these securely
APP_SECRET_KEY=<generate-with-openssl-rand-hex-32>
ADMIN_PASSWORD=<strong-password>

# Database connection
DATABASE_URL=postgresql://user:pass@localhost/readur
POSTGRES_PASSWORD=<secure-password>

# Document storage location
STORAGE_BACKEND=s3  # or 'local'
S3_BUCKET=readur-documents
S3_ACCESS_KEY_ID=<your-key>
S3_SECRET_ACCESS_KEY=<your-secret>
```

The APP_SECRET_KEY encrypts session cookies and other sensitive data - generate this using `openssl rand -hex 32` for security. Choose a strong admin password since this account has full system access. The database URL tells Readur how to connect to PostgreSQL, while storage settings determine where uploaded documents are kept.

[Complete configuration options →](../configuration-reference.md)

### Choosing Your Storage Backend

Readur supports two primary storage backends, each suited for different deployment scenarios. Your choice affects scalability, backup complexity, and operational requirements.

#### Local File Storage

Local storage keeps documents on the server's filesystem, which is simple to set up and backup:

```bash
STORAGE_BACKEND=local
LOCAL_STORAGE_PATH=/data/readur/documents
```

This approach works well for smaller deployments where the server has adequate storage capacity. Ensure the specified directory exists and is writable by the Readur process. Local storage makes backup straightforward - just include the document directory in your regular backup routine.

#### S3-Compatible Cloud Storage

S3 storage provides unlimited scalability and built-in redundancy, making it ideal for larger deployments:

```bash
STORAGE_BACKEND=s3
S3_ENDPOINT=https://s3.amazonaws.com
S3_BUCKET=my-readur-bucket
S3_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
S3_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
```

This configuration works with Amazon S3, or you can change the endpoint to use compatible services like MinIO, DigitalOcean Spaces, or Wasabi. Ensure your S3 credentials have appropriate permissions to create, read, and delete objects in the specified bucket.

[Detailed storage configuration →](./storage.md)

### Setting Up Authentication

Readur offers flexible authentication options to match your organizational needs. Choose local authentication for simple deployments or OIDC integration for enterprise environments with existing identity providers.

#### Local Authentication for Simple Deployments

Local authentication is straightforward and works well for smaller teams or personal installations:

```bash
AUTH_METHOD=local
ENABLE_REGISTRATION=false
REQUIRE_EMAIL_VERIFICATION=true
```

This configuration uses username and password authentication managed entirely within Readur. Disable registration to maintain control over who can create accounts, and enable email verification for additional security. You can always enable self-registration later if you want to allow users to create their own accounts.

#### Enterprise SSO Integration

For organizations with existing identity providers, OIDC integration provides seamless single sign-on:

```bash
AUTH_METHOD=oidc
OIDC_ISSUER=https://auth.company.com
OIDC_CLIENT_ID=readur
OIDC_CLIENT_SECRET=<secret>
```

This configuration connects Readur to your corporate identity provider (like Azure AD, Okta, or Keycloak). Users authenticate with their existing corporate credentials, and Readur automatically creates accounts on first login. You'll need to register Readur as an application in your identity provider and configure the client ID and secret.

[Complete authentication setup →](./authentication.md)

## Network Configuration

### Setting Up HTTPS with a Reverse Proxy

Production deployments should use HTTPS to protect user credentials and document content. NGINX provides a robust reverse proxy solution that handles SSL termination while forwarding requests to Readur:

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

This NGINX configuration listens on port 443 for HTTPS connections, terminates SSL encryption, and forwards requests to Readur running on port 8000. The proxy headers ensure Readur receives proper client information for logging and security. Replace the SSL certificate paths with your actual certificate files.

[Complete reverse proxy setup →](./reverse-proxy.md)

### Securing Network Access

Proper firewall configuration is essential for production deployments. Allow HTTPS traffic so users can access Readur securely, and restrict SSH access to known IP addresses for administrative access:

```bash
# Allow HTTPS traffic from anywhere
sudo ufw allow 443/tcp

# Allow SSH only from your administrative IP
sudo ufw allow from YOUR_IP to any port 22

# Activate the firewall with these rules
sudo ufw enable
```

Replace YOUR_IP with your actual administrative IP address. If you need to allow multiple administrative IPs, add additional rules for each one. Avoid allowing SSH from anywhere (0.0.0.0/0) as this creates unnecessary security exposure.

## Backup and Recovery

### Creating Automated Backup Routines

Regular backups are critical for any production system. This script creates daily backups of both your database and documents, then uploads them to secure storage with automatic cleanup:

```bash
#!/bin/bash
# backup.sh - Run daily via cron

# Create timestamped database backup
pg_dump $DATABASE_URL > backup-$(date +%Y%m%d).sql

# Backup documents if using local storage
tar -czf documents-$(date +%Y%m%d).tar.gz /data/readur/documents

# Upload backups to secure offsite storage
aws s3 cp backup-*.sql s3://backups/readur/
aws s3 cp documents-*.tar.gz s3://backups/readur/

# Remove local backups older than 30 days
find . -name "backup-*.sql" -mtime +30 -delete
```

Schedule this script to run daily using cron: `0 2 * * * /path/to/backup.sh`. The 2 AM timing avoids peak usage hours. If you're using S3 storage for documents, you only need to back up the database since S3 provides built-in redundancy.

[Complete backup strategy →](./backup.md)

### Disaster Recovery Procedures

When you need to restore from backup, follow these steps carefully to ensure complete recovery:

```bash
# Stop Readur services first
docker-compose down

# Restore database from backup
PGPASSWORD="${DB_PASSWORD}" psql -h localhost -U readur -d readur < backup-20240315.sql

# Restore documents (if using local storage)
tar -xzf documents-20240315.tar.gz -C /

# Restart services
docker-compose up -d

# Verify restoration by checking document count
docker-compose exec readur psql -U readur -d readur -c "SELECT COUNT(*) FROM documents;"
```

Always stop Readur services before restoration to prevent data corruption. After restoration, verify that the document count matches your expectations and test key functionality like search and document access.

## Security Hardening

### Essential Security Measures

Securing your Readur deployment requires attention to multiple layers of protection. Start with the fundamentals: change all default passwords immediately after installation, including the admin account and any service accounts. Enable HTTPS with valid SSL certificates - never run Readur in production over plain HTTP as this exposes user credentials and document content.

Configure firewall rules to limit network access to only required ports and sources. Disable any unnecessary services running on your server to reduce the attack surface. Enable audit logging to track access and changes, and consider implementing intrusion detection if your organization requires it.

Establish a regular security update schedule for both Readur and the underlying operating system. Implement rate limiting on your reverse proxy to protect against brute force attacks and resource exhaustion.

[Complete security hardening guide →](../security-guide.md)

### SSL Certificate Management

SSL certificates protect data in transit and establish trust with users. Let's Encrypt provides free, automated certificates that work well for most deployments:

```bash
# Obtain certificate for your domain
sudo certbot certonly --webroot -w /var/www/html -d readur.company.com

# Test automatic renewal
sudo certbot renew --dry-run
```

Let's Encrypt certificates expire every 90 days, but certbot automatically sets up renewal via cron. The dry-run test verifies that renewal will work when needed. For enterprise deployments, you might prefer certificates from your organization's existing CA.

## Monitoring and Maintenance

### Setting Up Health Monitoring

Readur provides several endpoints for monitoring system health and performance. Use these endpoints with your existing monitoring tools to track system status:

```bash
# Basic health check - returns 200 if system is operational
curl http://localhost:8000/health

# Detailed metrics in Prometheus format
curl http://localhost:8000/metrics

# Human-readable status information
curl http://localhost:8000/status
```

The health endpoint provides a simple up/down status check suitable for load balancer health checks. The metrics endpoint exports detailed performance data compatible with Prometheus and Grafana. The status endpoint gives human-readable information about queue status, database connectivity, and storage health.

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

[Update Procedures →](../migration-guide.md)

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

[Complete Troubleshooting Guide →](../troubleshooting.md)

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

[Migration Guide →](../migration-guide.md)

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