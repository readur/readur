# Docker Quick Start

Get Readur running on your machine in just 5 minutes using Docker. If you're familiar with Docker and want to test Readur quickly, this is your fastest path to a working installation.

## What You Need

Before starting, make sure you have Docker Engine 20.10 or newer and Docker Compose 2.0 or newer installed on your machine. You'll also need at least 2GB of RAM available for the containers and about 10GB of free disk space for the application and initial document storage.

## Step 1: Download Configuration Files

First, create a directory for Readur and download the configuration files you'll need:

```bash
# Create a directory for your Readur installation
mkdir readur && cd readur

# Download the Docker Compose configuration
curl -O https://raw.githubusercontent.com/readur/readur/main/docker-compose.yml

# Download the environment variables template
curl -O https://raw.githubusercontent.com/readur/readur/main/.env.example
mv .env.example .env
```

These commands download the Docker Compose file that defines all the services Readur needs, plus an environment template that you'll customize with your settings.

## Step 2: Configure Essential Settings

Open the `.env` file in your text editor and set up the minimal configuration needed to get started:

```bash
# Generate a secure secret key for the application
APP_SECRET_KEY=$(openssl rand -hex 32)

# Set a secure database password
POSTGRES_PASSWORD=$(openssl rand -hex 16)

# Set your admin password (change this from the default!)
ADMIN_PASSWORD=changeme123
```

These are the only settings you need to change for a basic installation. The secret key secures your application sessions, the database password protects your PostgreSQL instance, and the admin password is what you'll use to log in. Everything else uses sensible defaults that work for testing and development.

## Step 3: Start Readur

Now launch all the services that make up Readur:

```bash
# Start all containers in the background
docker-compose up -d
```

This command starts PostgreSQL (database), Redis (task queue), the main Readur application, and the OCR worker in the background. To watch the startup process and see what's happening:

```bash
# Follow the logs to see startup progress
docker-compose logs -f
```

The logs will show you when each service is ready. You'll see database initialization, the web server starting up, and the OCR worker connecting to the queue.

## Step 4: Access Readur

The startup process typically takes 30-60 seconds. Once the logs show that all services are ready, open your web browser and go to:

```
http://localhost:8000
```

Log in using the admin account:
- **Username:** `admin`
- **Password:** `changeme123` (or whatever you set in the .env file)

You'll be taken to the Readur dashboard where you can start uploading and managing documents immediately.

## Step 5: Test Document Processing

Now let's verify that everything is working by uploading a test document. The easiest way is through the web interface:

1. Click the **Upload** button in the Readur interface
2. Select a PDF, image, or text file from your computer
3. Make sure **OCR Processing** is enabled (it should be by default)
4. Click **Upload**

You'll see the document appear in your document list, and you can watch the OCR status change from "Pending" to "Processing" to "Completed." Once processing finishes, try searching for text you know is in the document to verify that text extraction worked correctly.

If you prefer to test via the API, you can also upload documents programmatically:

```bash
# Upload a document via the REST API
curl -X POST http://localhost:8000/api/upload \
  -H "Authorization: Bearer your-token" \
  -F "file=@test.pdf" \
  -F "ocr=true"
```

You'll need to get an API token first through the web interface if you want to use the API approach.

## Understanding the Docker Setup

Your Readur deployment consists of several containers working together:

- **readur** - The main application that handles the web interface, API, and document management
- **postgres** - The database that stores document metadata and search indexes
- **redis** - Manages the task queue for OCR processing and caches frequently accessed data
- **ocr-worker** - Dedicated service that processes documents and extracts text
- **nginx** - Optional web server for production deployments (not included in basic setup)

These services communicate with each other automatically through Docker's networking, so you don't need to configure connections between them.

## Managing Your Readur Installation

### Checking Service Status

To see which containers are running and their current status:

```bash
# View the status of all Readur services
docker-compose ps
```

This shows you which services are up, which ports they're using, and if any have stopped unexpectedly.

### Stopping and Starting

When you need to stop Readur temporarily (like for system maintenance):

```bash
# Stop all services but keep data
docker-compose stop
```

To start everything back up:

```bash
# Start all services again
docker-compose start
```

### Updating to Newer Versions

When a new version of Readur is released, update your installation:

```bash
# Download the latest container images
docker-compose pull

# Restart with the new versions
docker-compose up -d
```

### Complete Removal

If you want to completely remove Readur and all its data:

```bash
# CAUTION: This deletes all your documents and database!
docker-compose down -v
```

The `-v` flag removes the data volumes, which means you'll lose all uploaded documents and the database. Only use this if you're sure you want to start fresh.

## Backing Up Your Data

Your documents and database are stored in Docker volumes, which persist even when you stop or update containers. Here's how to back them up:

### List Your Data Volumes

```bash
# See all Docker volumes (look for ones starting with "readur_")
docker volume ls
```

### Backup the Database

```bash
# Create a backup of your PostgreSQL database
docker-compose exec postgres pg_dump -U readur > backup.sql
```

This creates a complete backup of your document metadata, search indexes, and user settings in a SQL file.

### Backup Document Files

```bash
# Create a backup of all uploaded documents
docker run --rm -v readur_documents:/data -v $(pwd):/backup \
  alpine tar czf /backup/documents.tar.gz /data
```

This command creates a compressed archive of all your uploaded documents. The backup file will appear in your current directory.

## Tuning Performance

If you want to optimize Readur for your specific hardware or usage patterns, you can adjust resource limits and storage locations.

### Setting Memory Limits

To prevent containers from using too much RAM, add resource limits to your `docker-compose.yml`:

```yaml
services:
  readur:
    mem_limit: 2g        # Limit main app to 2GB RAM
    memswap_limit: 2g    # Prevent swap usage
    
  ocr-worker:
    mem_limit: 1g        # OCR worker gets 1GB RAM
    cpus: '2.0'          # Use up to 2 CPU cores
```

These limits help prevent Readur from overwhelming your system, especially on shared servers.

### Custom Storage Locations

By default, Docker stores volumes in its own directories. To use a specific location for documents:

```yaml
volumes:
  documents:
    driver: local
    driver_opts:
      type: none
      device: /mnt/storage/readur  # Your custom path
      o: bind
```

This is useful when you have a dedicated disk or mounted network storage for documents.

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