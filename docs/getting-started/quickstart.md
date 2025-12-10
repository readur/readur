# Quick Start Guide

Get Readur running and process your first documents in under 5 minutes.

## Prerequisites

Ensure you have Docker and Docker Compose installed:
```bash
docker --version  # Should be 20.10+
docker-compose --version  # Should be 2.0+
```

## 5-Minute Setup

### Step 1: Get Readur

```bash
# Clone and enter the repository
git clone https://github.com/readur/readur.git
cd readur
```

### Step 2: Start Services

```bash
# Start with default configuration
docker-compose up -d

# Watch the logs (optional)
docker-compose logs -f
```

Wait about 30 seconds for services to initialize.

### Step 3: Access the Interface

Open your browser and navigate to:
```
http://localhost:8000
```

Login with your admin credentials:
- **Username**: `admin`
- **Password**: Check the container logs for the auto-generated password

On first startup, Readur generates a secure admin password and displays it in the logs. View the logs with `docker-compose logs` and look for the "READUR ADMIN USER CREATED" section. Save this password immediately - it won't be shown again.

### Step 4: Upload Your First Document

#### Via Web Interface

Now you can test Readur's core functionality by uploading a document. Click the **Upload** button in the top navigation, then drag and drop a PDF or image file onto the upload area. After clicking **Upload** to start processing, you'll see the document appear in your document list with a status indicator showing OCR progress. Wait for the indicator to turn green, which means text extraction is complete and the document is searchable.

#### Via API (Optional)

If you prefer working with APIs or want to automate document uploads, you can use Readur's REST API. First, authenticate to get an access token:

```bash
# Authenticate and get a session token (use your generated password from the logs)
TOKEN=$(curl -s -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"YOUR_GENERATED_PASSWORD"}' | jq -r .token)

# Upload a document using the API
curl -X POST http://localhost:8000/api/documents/upload \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@your-document.pdf"
```

This approach is particularly useful for integrating Readur with other systems or creating automated document processing workflows.

### Step 5: Search Your Documents

Once the OCR indicator shows green (processing complete), you can test Readur's search capabilities. Use the search bar at the top of any page and enter text you know exists in your uploaded document. Press Enter to see search results, which will show document snippets containing your search terms. Click on any result to view the full document with your search terms highlighted.

## Common First Tasks

### Resetting Admin Password

If you lose your admin password or need to reset it, you can use the built-in CLI command:

```bash
docker exec readur readur reset-admin-password
```

This generates a new secure password and displays it. You can also set a specific password using the `ADMIN_PASSWORD` environment variable.

### Add Your First Source

While manual uploads work fine for testing, most users benefit from setting up automatic document imports. Go to **Settings** → **Sources** and click **Add Source** to configure external storage integration. Choose Local Folder if you want to monitor directories on your server, WebDAV for cloud services like Nextcloud or ownCloud, or S3 for cloud object storage.

Each source type requires different connection details - local folders need directory paths, WebDAV sources need server URLs and credentials, while S3 sources require bucket names and access keys. Always test your connection before saving to ensure Readur can access your storage successfully.

### Create Document Labels

Labels help organize your growing document collection by letting you assign categories and tags. Navigate to **Settings** → **Labels**, click **Create Label**, enter a descriptive name, and choose a color for visual identification. Save the label to make it available throughout Readur.

You can apply labels in several ways: from individual document detail pages when reviewing content, through bulk selection when organizing multiple documents at once, or during the upload process to categorize documents immediately. Consistent labeling makes finding documents much easier as your collection grows.

### Set Up Watch Folder

Watch folders provide automatic document import when files are dropped into specific directories. Create a watch directory and configure Docker to mount it into your Readur container:

```bash
# Create a local watch directory
mkdir -p ./data/watch

# Add this volume mapping to your docker-compose.yml:
volumes:
  - ./data/watch:/app/watch

# Restart Readur to apply the new volume
docker-compose restart readur
```

Once configured, any files dropped into `./data/watch` will be automatically imported and processed. This setup is perfect for scanner integration or automated document workflows.

## Essential Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `/` or `Ctrl+K` | Focus search bar |
| `Ctrl+U` | Open upload dialog |
| `Esc` | Close dialogs |
| `G then D` | Go to documents |
| `G then S` | Go to settings |

## Sample Workflow

### Legal Document Management

Here's how a law firm might configure Readur for contract and invoice management:

```bash
# Create an organized label structure
Labels: "Contracts", "Invoices", "Legal", "2024"

# Connect to existing document storage
Source: /shared/legal-docs (WebDAV)
Sync: Every 30 minutes

# Optimize OCR for legal documents
Language: English
Quality: High
Concurrent Jobs: 4

# Process existing documents
Select all PDFs → Upload → Apply "2024" label

# Create smart collections for quick access
Search: label:Contracts AND date:2024
Save as: "2024 Contracts"
```

This setup automatically imports new documents from shared storage, processes them with high-quality OCR, and organizes them with consistent labeling.

### Research Paper Archive

Academic researchers can configure Readur to handle multilingual papers and complex search needs:

```bash
# Configure for multilingual academic content
OCR Language: Multiple (eng+deu+fra)
Max File Size: 100MB

# Create research-focused categories
Labels: "Published", "Draft", "Review", "Citations"

# Set up automated import from research directories
Watch Folder: /research/papers
Process: Auto-OCR and label by folder

# Enable advanced search for academic work
Boolean search: enabled
Fuzzy matching: 2 (for OCR errors)
```

This configuration handles papers in multiple languages, supports large files common in academic work, and provides sophisticated search capabilities for research workflows.

## Performance Tips

### Optimizing OCR Processing Speed

If you have adequate CPU resources, increase concurrent processing to handle multiple documents simultaneously:

```bash
# Increase concurrent jobs to match your CPU cores
CONCURRENT_OCR_JOBS=8

# Optimize for your specific document types
OCR_LANGUAGE=eng  # Single language is faster than auto-detection
ENABLE_PREPROCESSING=false  # Skip for clean, well-scanned documents
```

These settings work best when your server has multiple CPU cores and you're processing documents with consistent quality and language.

### Scaling for Large Document Collections

For organizations with thousands of documents, cloud storage and increased resources improve performance:

```bash
# Use cloud storage for unlimited capacity
S3_ENABLED=true
S3_BUCKET_NAME=readur-docs

# Allocate more memory for large processing batches
MEMORY_LIMIT_MB=4096

# Enable compression to reduce storage costs
ENABLE_COMPRESSION=true
```

S3 storage eliminates local disk space constraints, while increased memory limits allow processing larger batches of documents efficiently.

## Troubleshooting Quick Fixes

### OCR Not Starting
```bash
# Check the queue
curl http://localhost:8000/api/admin/queue/status

# Restart OCR workers
docker-compose restart readur
```

### Can't Login
```bash
# Reset to default password
docker exec readur python reset_admin_password.py
```

### Slow Search
```bash
# Rebuild search index
docker exec readur python rebuild_index.py
```

## Next Steps

Now that you have Readur running:

1. **[Configure OCR](../multi-language-ocr-guide.md)** for your language
2. **[Set up Sources](../sources-guide.md)** for automated import
3. **[Create Labels](../labels-and-organization.md)** for organization
4. **[Learn Advanced Search](../advanced-search.md)** techniques
5. **[Configure Backups](../deployment.md#backup-strategy)** for data safety

## Getting Help

- **Documentation**: [Full User Guide](../user-guide.md)
- **API Reference**: [REST API Docs](../api-reference.md)
- **Community**: [GitHub Discussions](https://github.com/readur/readur/discussions)
- **Issues**: [Report Bugs](https://github.com/readur/readur/issues)