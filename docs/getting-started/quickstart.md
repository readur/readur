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

Login with default credentials:
- **Username**: `admin`
- **Password**: `readur2024`

### Step 4: Upload Your First Document

#### Via Web Interface

1. Click the **Upload** button in the top navigation
2. Drag and drop a PDF or image file
3. Click **Upload** to start processing
4. Wait for the OCR indicator to turn green

#### Via API (Optional)

```bash
# Get authentication token
TOKEN=$(curl -s -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"readur2024"}' | jq -r .token)

# Upload a document
curl -X POST http://localhost:8000/api/documents/upload \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@your-document.pdf"
```

### Step 5: Search Your Documents

Once OCR processing completes (green indicator):

1. Use the **Search** bar at the top
2. Enter any text from your document
3. Press Enter to see results
4. Click on a result to view the document

## Common First Tasks

### Change Admin Password

**Important**: Do this immediately after installation.

1. Navigate to **Settings** → **User Management**
2. Click on the admin user
3. Enter a new secure password
4. Click **Save**

### Add Your First Source

Automatically import documents from external storage:

1. Go to **Settings** → **Sources**
2. Click **Add Source**
3. Choose your source type:
   - **Local Folder**: For directories on the server
   - **WebDAV**: For Nextcloud/ownCloud
   - **S3**: For cloud storage
4. Configure connection details
5. Test and save

### Create Document Labels

Organize your documents with labels:

1. Navigate to **Settings** → **Labels**
2. Click **Create Label**
3. Enter a name and choose a color
4. Save the label
5. Apply to documents via:
   - Document details page
   - Bulk selection
   - During upload

### Set Up Watch Folder

Monitor a directory for automatic document import:

```bash
# Create a watch directory
mkdir -p ./data/watch

# Add to docker-compose.yml volumes:
volumes:
  - ./data/watch:/app/watch

# Restart Readur
docker-compose restart readur
```

Drop files into `./data/watch` - they'll be automatically imported.

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

```bash
# 1. Create label structure
Labels: "Contracts", "Invoices", "Legal", "2024"

# 2. Set up source folder
Source: /shared/legal-docs (WebDAV)
Sync: Every 30 minutes

# 3. Configure OCR
Language: English
Quality: High
Concurrent Jobs: 4

# 4. Upload initial batch
Select all PDFs → Upload → Apply "2024" label

# 5. Create saved search
Search: label:Contracts AND date:2024
Save as: "2024 Contracts"
```

### Research Paper Archive

```bash
# 1. Configure for academic documents
OCR Language: Multiple (eng+deu+fra)
Max File Size: 100MB

# 2. Create categories
Labels: "Published", "Draft", "Review", "Citations"

# 3. Set up automated import
Watch Folder: /research/papers
Process: Auto-OCR and label by folder

# 4. Advanced search setup
Boolean search: enabled
Fuzzy matching: 2 (for OCR errors)
```

## Performance Tips

### For Faster OCR Processing

```bash
# Increase concurrent jobs (if you have CPU cores)
CONCURRENT_OCR_JOBS=8

# Optimize for your document types
OCR_LANGUAGE=eng  # Single language is faster
ENABLE_PREPROCESSING=false  # Skip if documents are clean
```

### For Large Document Collections

```bash
# Use S3 storage instead of local
S3_ENABLED=true
S3_BUCKET_NAME=readur-docs

# Increase memory limits
MEMORY_LIMIT_MB=4096

# Enable compression
ENABLE_COMPRESSION=true
```

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