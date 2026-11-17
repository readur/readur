# Quick Start Guide

Get Readur running with Docker Compose and process your first document.

## Prerequisites

Docker with the Compose plugin:

```bash
docker --version
docker compose version
```

## Setup

### Step 1: Get Readur

```bash
git clone https://github.com/readur/readur.git
cd readur
```

### Step 2: Start the Services

```bash
# Optional: choose the initial admin password (at least 8 characters).
# Without it, a random password is generated (see Step 3).
echo "ADMIN_PASSWORD=choose-a-long-password" > .env

# Build the image and start PostgreSQL and Readur
docker compose up -d --build

# Follow the logs (Ctrl+C stops following)
docker compose logs -f readur
```

The first build compiles Readur from source and takes a while. Readur is ready when the log shows `✅ HTTP server bound to: 0.0.0.0:8000`. To skip the build, use the published image as described in the [Docker Quick Start](../quickstart/docker.md).

### Step 3: Sign In

Open `http://localhost:8000` and sign in:

- **Username**: `admin`
- **Password**: your `ADMIN_PASSWORD`, or the generated password

Without `ADMIN_PASSWORD`, Readur generates a random admin password on first startup and writes it to `/app/uploads/.readur/initial-admin-password` inside the container (it is not written to the logs):

```bash
docker compose exec readur cat /app/uploads/.readur/initial-admin-password
```

After signing in and changing the password, delete the file.

### Step 4: Upload Your First Document

#### Via the Web Interface

Open **Intake** in the navigation and choose **Upload**, then drop a PDF, image or text file onto the upload area. The document is stored and queued for OCR; it appears in the **Library**, and once OCR has finished its text is searchable.

#### Via the API

```bash
# Log in and keep the session token
token=$(curl -s -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"your-admin-password"}' | jq -r .token)

# Upload a document
curl -X POST http://localhost:8000/api/documents \
  -H "Authorization: Bearer $token" \
  -F "file=@your-document.pdf"
```

For scripts and integrations, create an API key under **Settings** → **API Keys** instead of logging in with a password. See the [API Reference](../api-reference.md).

### Step 5: Search

Once OCR has finished, search for a word you know is in the document, from the search box or the **Advanced search** page.

## Common First Tasks

### Reset the Admin Password

```bash
docker compose exec readur /app/readur reset-admin-password
```

This prints a new random password, or sets the password to `ADMIN_PASSWORD` when that is set in the container's environment.

### Add a Source

Under **Sources**, add a WebDAV server (such as Nextcloud or ownCloud), an S3 bucket, or a folder on the server, and Readur imports its documents on a schedule. Test the connection before saving. See the [Sources Guide](../sources-guide.md).

### Create Labels

Under **Settings** → **Labels**, create labels with a name and colour. You can attach labels to a single document, to several selected documents at once, or while uploading. See [Labels and Organization](../labels-and-organization.md).

### Use the Watch Folder

The compose file mounts `./readur_watch` on the host as the watch folder `/app/watch`. Files copied there are imported automatically once they have stopped changing:

```bash
cp ~/scans/invoice.pdf ./readur_watch/
```

Only the extensions in `ALLOWED_FILE_TYPES` (default `pdf,txt,doc,docx,png,jpg,jpeg`) are imported, and the repository's compose file sets `MAX_FILE_AGE_HOURS: 24`, so files last modified more than a day ago are skipped. See [Watch Folder](../WATCH_FOLDER.md).

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+K` (`⌘K` on macOS) | Open the command palette |
| `/` | Focus the library search box, or open the command palette elsewhere |
| `Esc` | Close the command palette or a dialog |

## Performance Tips

OCR throughput is set by `CONCURRENT_OCR_JOBS` (default `4`, at most 15) in the `environment:` section of `docker-compose.yml`. Raise it on machines with more CPU cores and memory; lower it on small machines. A single OCR language is faster than several.

To store documents in S3 instead of the local `./readur_uploads` directory, see the [S3 Storage Guide](../s3-storage-guide.md).

## Troubleshooting

### OCR Not Starting

Check the logs, and as an administrator the OCR queue:

```bash
docker compose logs readur | grep -i ocr
curl -H "Authorization: Bearer $token" http://localhost:8000/api/queue/stats
```

### Can't Sign In

Reset the admin password as shown in [Reset the Admin Password](#reset-the-admin-password).

## Next Steps

1. [Multi-Language OCR](../multi-language-ocr-guide.md)
2. [Sources Guide](../sources-guide.md)
3. [Labels and Organization](../labels-and-organization.md)
4. [Advanced Search](../advanced-search.md)
5. [Backups](../deployment.md#backup-strategy)

## Getting Help

- [User Guide](../user-guide.md)
- [GitHub Discussions](https://github.com/readur/readur/discussions)
- [GitHub Issues](https://github.com/readur/readur/issues)
