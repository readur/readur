# Per-User Watch Directories Documentation

## Table of Contents

1. [Overview](#overview)
2. [Architecture and Components](#architecture-and-components)
3. [Prerequisites and Requirements](#prerequisites-and-requirements)
4. [Administrator Setup Guide](#administrator-setup-guide)
5. [User Guide](#user-guide)
6. [API Reference](#api-reference)
7. [Configuration Reference](#configuration-reference)
8. [Security Considerations](#security-considerations)
9. [Troubleshooting](#troubleshooting)
10. [Examples and Best Practices](#examples-and-best-practices)

## Overview

The Per-User Watch Directories feature in Readur allows each user to have their own dedicated folder for automatic document ingestion. When enabled, documents placed in a user's watch directory are automatically processed, OCR'd, and associated with that specific user's account.

### Key Benefits

- **User Isolation**: Each user's documents remain private and separate
- **Automatic Attribution**: Documents are automatically assigned to the correct user
- **Simplified Workflow**: Users can drop files into their folder without manual upload
- **Batch Processing**: Process multiple documents simultaneously
- **Integration Support**: Works with network shares, sync tools, and automated workflows

### How It Works

1. Administrator enables per-user watch directories in configuration
2. System creates a dedicated folder for each user (e.g., `/data/user_watch/username/`)
3. Users place documents in their watch folder
4. Readur's file watcher detects new files
5. Documents are automatically ingested and associated with the user
6. OCR processing extracts text for searching
7. Documents appear in the user's library

## Architecture and Components

### System Components

1. **UserWatchService** (`src/services/user_watch_service.rs`)
   - Manages user-specific watch directories
   - Handles directory creation, validation, and cleanup
   - Provides secure path operations

2. **UserWatchManager** (`src/scheduling/user_watch_manager.rs`)
   - Coordinates between file watcher and user management
   - Maps file paths to users
   - Manages user cache for performance

3. **File Watcher** (`src/scheduling/watcher.rs`)
   - Monitors both global and per-user directories
   - Determines file ownership based on directory location
   - Triggers document ingestion pipeline

4. **API Endpoints** (`src/routes/users.rs`)
   - REST API for managing user watch directories
   - Provides status, creation, and deletion operations

### Directory Structure

```
user_watch_base_dir/           # Base directory (configurable)
├── alice/                     # User alice's watch directory
│   ├── document1.pdf
│   └── report.docx
├── bob/                       # User bob's watch directory
│   └── invoice.pdf
└── charlie/                   # User charlie's watch directory
    ├── presentation.pptx
    └── notes.txt
```

## Prerequisites and Requirements

### System Requirements

- **Operating System**: Linux, macOS, or Windows with proper file permissions
- **Storage**: Sufficient disk space for user directories and documents
- **File System**: Support for directory permissions (recommended: ext4, NTFS, APFS)
- **Readur Version**: 2.5.4 or later

### Software Requirements

- PostgreSQL database
- Readur server with file watching enabled
- Proper file system permissions for the Readur process

### Network Requirements (Optional)

- Network file system support (NFS, SMB/CIFS) for remote directories
- Stable network connection for remote file access

## Administrator Setup Guide

### Step 1: Enable Per-User Watch Directories

Edit your `.env` file or set environment variables:

```bash
# Enable the feature
ENABLE_PER_USER_WATCH=true

# Set the base directory for user watch folders
USER_WATCH_BASE_DIR=/data/user_watch

# Configure watch interval (optional, default: 60 seconds)
WATCH_INTERVAL_SECONDS=30

# Set file stability check (optional, default: 2000ms)
FILE_STABILITY_CHECK_MS=3000

# Set maximum file age to process (optional, default: 24 hours)
MAX_FILE_AGE_HOURS=48
```

### Step 2: Create Base Directory

Ensure the base directory exists with proper permissions:

```bash
# Create the base directory
sudo mkdir -p /data/user_watch

# Set ownership to the user running Readur
sudo chown readur:readur /data/user_watch

# Set permissions (owner: read/write/execute, group: read/execute)
sudo chmod 755 /data/user_watch
```

### Step 3: Configure Directory Permissions

For production environments, configure appropriate permissions:

```bash
# Option 1: Shared group access
sudo groupadd readur-users
sudo usermod -a -G readur-users readur
sudo chgrp -R readur-users /data/user_watch
sudo chmod -R 2775 /data/user_watch  # SGID bit ensures new files inherit group

# Option 2: ACL-based permissions (more granular)
sudo setfacl -R -m u:readur:rwx /data/user_watch
sudo setfacl -R -d -m u:readur:rwx /data/user_watch
```

### Step 4: Network Share Setup (Optional)

To allow users to access their watch directories via network shares:

#### SMB/CIFS Share Configuration

```ini
# /etc/samba/smb.conf
[readur-watch]
   path = /data/user_watch
   valid users = @readur-users
   writable = yes
   browseable = yes
   create mask = 0660
   directory mask = 0770
   force group = readur-users
```

#### NFS Export Configuration

```bash
# /etc/exports
/data/user_watch *(rw,sync,no_subtree_check,no_root_squash)
```

### Step 5: Restart Readur

After configuration, restart the Readur service:

```bash
# Systemd
sudo systemctl restart readur

# Docker
docker-compose restart readur

# Direct execution
# Stop the current process and start with new configuration
```

### Step 6: Verify Configuration

Check the Readur logs to confirm per-user watch is enabled:

```bash
# Check logs for confirmation
grep "Per-user watch enabled" /var/log/readur/readur.log

# Expected output:
# ✅ Per-user watch enabled: true
# 📂 User watch base directory: /data/user_watch
```

## User Guide

### Accessing Your Watch Directory

#### Method 1: Direct File System Access

If you have direct access to the server:

```bash
# Navigate to your watch directory
cd /data/user_watch/your-username/

# Copy files
cp ~/Documents/*.pdf /data/user_watch/your-username/

# Move files
mv ~/Downloads/report.docx /data/user_watch/your-username/
```

#### Method 2: Network Share Access

Access via SMB/CIFS on Windows:

1. Open File Explorer
2. Type in address bar: `\\server-name\readur-watch\your-username`
3. Drag and drop files into your folder

Access via SMB/CIFS on macOS:

1. Open Finder
2. Press Cmd+K
3. Enter: `smb://server-name/readur-watch/your-username`
4. Drag and drop files into your folder

#### Method 3: Sync Tools

Use synchronization tools for automatic uploads:

```bash
# Using rsync
rsync -avz ~/Documents/*.pdf server:/data/user_watch/your-username/

# Using rclone
rclone copy ~/Documents server:user_watch/your-username/

# Using Syncthing (configure folder sync)
# Add /data/user_watch/your-username as a sync folder
```

### Managing Your Watch Directory via Web Interface

1. **Check Directory Status**
   - Navigate to Settings → Watch Folder
   - View your watch directory path and status
   - See if directory exists and is enabled

2. **Create Your Directory**
   - Click "Create Watch Directory" button
   - System will create your personal folder
   - Confirmation message will appear

3. **View Directory Path**
   - Your directory path is displayed
   - Copy path for reference
   - Share with IT for network access setup

### Supported File Types

Place any of these file types in your watch directory:

- **Documents**: PDF, TXT, DOC, DOCX, ODT, RTF
- **Images**: PNG, JPG, JPEG, TIFF, BMP
- **Presentations**: PPT, PPTX, ODP
- **Spreadsheets**: XLS, XLSX, ODS

### File Processing Workflow

1. **File Detection**: System checks for new files every 30-60 seconds
2. **Stability Check**: Waits for file to stop changing (2-3 seconds)
3. **Validation**: Verifies file type and size
4. **Ingestion**: Creates document record in database
5. **OCR Queue**: Adds to processing queue
6. **Text Extraction**: OCR processes the document
7. **Search Index**: Document becomes searchable

### Best Practices for Users

1. **File Naming**: Use descriptive names for easier identification
2. **File Size**: Keep files under 50MB for optimal processing
3. **Batch Upload**: Can upload multiple files simultaneously
4. **Organization**: Create subfolders within your watch directory
5. **Patience**: Allow 1-5 minutes for processing depending on file size

## API Reference

### Get User Watch Directory Information

Retrieve information about a user's watch directory.

**Endpoint**: `GET /api/users/{user_id}/watch-directory`

**Headers**:
```http
Authorization: Bearer {jwt_token}
```

**Response** (200 OK):
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "username": "alice",
  "watch_directory_path": "/data/user_watch/alice",
  "exists": true,
  "enabled": true
}
```

**Error Responses**:
- `401 Unauthorized`: Missing or invalid authentication
- `403 Forbidden`: Insufficient permissions
- `404 Not Found`: User not found
- `500 Internal Server Error`: Per-user watch disabled

### Create User Watch Directory

Create or ensure a user's watch directory exists.

**Endpoint**: `POST /api/users/{user_id}/watch-directory`

**Headers**:
```http
Authorization: Bearer {jwt_token}
Content-Type: application/json
```

**Request Body**:
```json
{
  "ensure_created": true
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "message": "Watch directory ready for user 'alice'",
  "watch_directory_path": "/data/user_watch/alice"
}
```

**Error Responses**:
- `401 Unauthorized`: Missing or invalid authentication
- `403 Forbidden`: Insufficient permissions
- `404 Not Found`: User not found
- `500 Internal Server Error`: Creation failed or feature disabled

### Delete User Watch Directory

Remove a user's watch directory and its contents.

**Endpoint**: `DELETE /api/users/{user_id}/watch-directory`

**Headers**:
```http
Authorization: Bearer {jwt_token}
```

**Note**: Only administrators can delete watch directories.

**Response** (200 OK):
```json
{
  "success": true,
  "message": "Watch directory removed for user 'alice'",
  "watch_directory_path": null
}
```

**Error Responses**:
- `401 Unauthorized`: Missing or invalid authentication
- `403 Forbidden`: Admin access required
- `404 Not Found`: User not found
- `500 Internal Server Error`: Deletion failed

### API Usage Examples

#### Python Example

```python
import requests

# Configuration
base_url = "https://readur.example.com/api"
token = "your-jwt-token"
user_id = "550e8400-e29b-41d4-a716-446655440000"

headers = {
    "Authorization": f"Bearer {token}",
    "Content-Type": "application/json"
}

# Get watch directory info
response = requests.get(
    f"{base_url}/users/{user_id}/watch-directory",
    headers=headers
)
info = response.json()
print(f"Watch directory: {info['watch_directory_path']}")
print(f"Exists: {info['exists']}")

# Create watch directory
response = requests.post(
    f"{base_url}/users/{user_id}/watch-directory",
    headers=headers,
    json={"ensure_created": True}
)
result = response.json()
if result['success']:
    print(f"Created: {result['watch_directory_path']}")
```

#### JavaScript/TypeScript Example

```typescript
// Using the provided API service
import { userWatchService } from './services/api';

// Get watch directory information
const getWatchInfo = async (userId: string) => {
  try {
    const response = await userWatchService.getUserWatchDirectory(userId);
    console.log('Watch directory:', response.data.watch_directory_path);
    console.log('Exists:', response.data.exists);
    return response.data;
  } catch (error) {
    console.error('Failed to get watch directory info:', error);
  }
};

// Create watch directory
const createWatchDirectory = async (userId: string) => {
  try {
    const response = await userWatchService.createUserWatchDirectory(userId);
    if (response.data.success) {
      console.log('Created:', response.data.watch_directory_path);
    }
    return response.data;
  } catch (error) {
    console.error('Failed to create watch directory:', error);
  }
};
```

#### cURL Examples

```bash
# Get watch directory information
curl -X GET "https://readur.example.com/api/users/${USER_ID}/watch-directory" \
  -H "Authorization: Bearer ${TOKEN}"

# Create watch directory
curl -X POST "https://readur.example.com/api/users/${USER_ID}/watch-directory" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{"ensure_created": true}'

# Delete watch directory (admin only)
curl -X DELETE "https://readur.example.com/api/users/${USER_ID}/watch-directory" \
  -H "Authorization: Bearer ${TOKEN}"
```

## Configuration Reference

### Environment Variables

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `ENABLE_PER_USER_WATCH` | Boolean | `false` | Enable/disable per-user watch directories |
| `USER_WATCH_BASE_DIR` | String | `./user_watch` | Base directory for all user watch folders |
| `WATCH_INTERVAL_SECONDS` | Integer | `60` | How often to scan for new files (seconds) |
| `FILE_STABILITY_CHECK_MS` | Integer | `2000` | Time to wait for file size stability (milliseconds) |
| `MAX_FILE_AGE_HOURS` | Integer | `24` | Maximum age of files to process (hours) |

### Configuration Validation

The system performs several validation checks:

1. **Path Validation**: Ensures paths are distinct and non-overlapping
2. **Directory Conflicts**: Prevents USER_WATCH_BASE_DIR from being:
   - The same as UPLOAD_PATH
   - The same as WATCH_FOLDER
   - Inside UPLOAD_PATH
   - Containing UPLOAD_PATH

### Docker Configuration

When using Docker, mount the user watch directory:

```yaml
version: '3.8'

services:
  readur:
    image: readur:latest
    environment:
      - ENABLE_PER_USER_WATCH=true
      - USER_WATCH_BASE_DIR=/app/user_watch
      - WATCH_INTERVAL_SECONDS=30
    volumes:
      - ./user_watch:/app/user_watch
      - ./uploads:/app/uploads
      - ./watch:/app/watch
    ports:
      - "8000:8000"
```

### Kubernetes Configuration

For Kubernetes deployments:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: readur-config
data:
  ENABLE_PER_USER_WATCH: "true"
  USER_WATCH_BASE_DIR: "/data/user_watch"
  WATCH_INTERVAL_SECONDS: "30"
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: readur
spec:
  template:
    spec:
      containers:
      - name: readur
        image: readur:latest
        envFrom:
        - configMapRef:
            name: readur-config
        volumeMounts:
        - name: user-watch
          mountPath: /data/user_watch
      volumes:
      - name: user-watch
        persistentVolumeClaim:
          claimName: readur-user-watch-pvc
```

## Security Considerations

### Username Validation

The system enforces strict username validation to prevent security issues:

- **Length**: 1-64 characters
- **Allowed Characters**: Alphanumeric, underscore (_), dash (-)
- **Prohibited Patterns**: 
  - Path traversal attempts (.., /)
  - Hidden directories (starting with .)
  - Null bytes or special characters

### Directory Permissions

1. **User Isolation**: Each user's directory is separate
2. **Permission Model**: 755 (owner: rwx, group: r-x, others: r-x)
3. **Ownership**: Readur process owns all directories
4. **SGID Bit**: Optional for group inheritance

### Path Security

- **Canonicalization**: All paths are canonicalized to prevent traversal
- **Boundary Checking**: Files must be within designated directories
- **Validation**: Extracted usernames are validated before use

### Access Control

- **API Protection**: JWT authentication required
- **Permission Levels**:
  - Users: Can only access their own directory
  - Admins: Can manage all directories
- **Directory Creation**: Users can create their own, admins can create any
- **Directory Deletion**: Admin-only operation

### Audit Considerations

1. **Logging**: All directory operations are logged
2. **File Attribution**: Documents tracked to source user
3. **Access Tracking**: API access logged with user context

## Troubleshooting

### Common Issues and Solutions

#### Issue: Per-user watch directories not working

**Symptoms**: Files in user directories are not processed

**Solutions**:
1. Verify feature is enabled:
   ```bash
   grep ENABLE_PER_USER_WATCH .env
   # Should show: ENABLE_PER_USER_WATCH=true
   ```

2. Check base directory exists and has correct permissions:
   ```bash
   ls -la /data/user_watch
   # Should show readur as owner with 755 permissions
   ```

3. Review logs for errors:
   ```bash
   grep -i "user watch" /var/log/readur/readur.log
   ```

#### Issue: "User watch service not initialized" error

**Symptoms**: API returns 500 error when accessing watch directories

**Solutions**:
1. Ensure ENABLE_PER_USER_WATCH=true in configuration
2. Restart Readur service
3. Check initialization logs for errors

#### Issue: Files not being detected

**Symptoms**: Files placed in watch directory are not processed

**Solutions**:
1. Check file permissions:
   ```bash
   ls -la /data/user_watch/username/
   # Files should be readable by readur user
   ```

2. Verify file type is supported:
   ```bash
   echo $ALLOWED_FILE_TYPES
   # Ensure your file extension is included
   ```

3. Check file age restriction:
   ```bash
   # Files older than MAX_FILE_AGE_HOURS are ignored
   find /data/user_watch -type f -mtime +1
   ```

#### Issue: Permission denied errors

**Symptoms**: Users cannot write to their watch directories

**Solutions**:
1. Fix directory ownership:
   ```bash
   sudo chown -R readur:readur /data/user_watch
   ```

2. Set correct permissions:
   ```bash
   sudo chmod -R 755 /data/user_watch
   ```

3. For shared access, use group permissions:
   ```bash
   sudo chmod -R 775 /data/user_watch
   sudo chgrp -R readur-users /data/user_watch
   ```

#### Issue: Duplicate documents created

**Symptoms**: Same file creates multiple documents

**Solutions**:
1. Ensure file stability check is adequate:
   ```bash
   # Increase if files are still being written
   FILE_STABILITY_CHECK_MS=5000
   ```

2. Check for file system issues (timestamps, inode changes)
3. Review deduplication settings in configuration

### Diagnostic Commands

```bash
# Check if user watch is enabled
curl -H "Authorization: Bearer $TOKEN" \
  https://readur.example.com/api/users/$USER_ID/watch-directory

# List all user directories
ls -la /data/user_watch/

# Check file watcher logs
journalctl -u readur | grep -i "watch"

# Monitor file processing in real-time
tail -f /var/log/readur/readur.log | grep -E "(Processing new file|watch)"

# Check directory permissions
namei -l /data/user_watch/username/

# Find recently modified files
find /data/user_watch -type f -mmin -60

# Check disk space
df -h /data/user_watch
```

## Examples and Best Practices

### Example 1: Small Team Setup

For a team of 5-10 users with local file access:

```bash
# .env configuration
ENABLE_PER_USER_WATCH=true
USER_WATCH_BASE_DIR=/srv/readur/user_watches
WATCH_INTERVAL_SECONDS=60
FILE_STABILITY_CHECK_MS=2000
MAX_FILE_AGE_HOURS=72

# Directory structure
/srv/readur/user_watches/
├── alice/
├── bob/
├── charlie/
├── diana/
└── edward/
```

### Example 2: Enterprise Network Share Integration

For larger organizations with network shares:

```bash
# Mount network share
sudo mount -t cifs //fileserver/readur /mnt/readur \
  -o username=readur,domain=COMPANY

# .env configuration
ENABLE_PER_USER_WATCH=true
USER_WATCH_BASE_DIR=/mnt/readur/user_watches
WATCH_INTERVAL_SECONDS=120  # Slower for network
FILE_STABILITY_CHECK_MS=5000  # Higher for network delays
```

### Example 3: Automated Document Workflow

Script for automatic document routing:

```python
#!/usr/bin/env python3
"""
Auto-route documents to user watch directories based on metadata
"""
import os
import shutil
from pathlib import Path

def route_document(file_path, user_mapping):
    """Route document to appropriate user watch directory"""
    
    # Extract metadata (example: from filename)
    filename = os.path.basename(file_path)
    
    # Determine target user (implement your logic)
    if "invoice" in filename.lower():
        target_user = "accounting"
    elif "report" in filename.lower():
        target_user = "management"
    else:
        target_user = "general"
    
    # Move to user's watch directory
    user_watch_dir = Path(f"/data/user_watch/{target_user}")
    if user_watch_dir.exists():
        dest = user_watch_dir / filename
        shutil.move(file_path, dest)
        print(f"Moved {filename} to {target_user}'s watch directory")
    else:
        print(f"User {target_user} watch directory does not exist")

# Monitor incoming directory
incoming_dir = Path("/srv/incoming")
for file_path in incoming_dir.glob("*.pdf"):
    route_document(file_path, user_mapping={})
```

### Example 4: Bulk User Setup

PowerShell script for creating multiple user directories:

```powershell
# bulk-create-watch-dirs.ps1
$baseUrl = "https://readur.example.com/api"
$adminToken = "your-admin-token"

$users = @("alice", "bob", "charlie", "diana", "edward")

foreach ($username in $users) {
    # Get user ID
    $userResponse = Invoke-RestMethod `
        -Uri "$baseUrl/users" `
        -Headers @{Authorization="Bearer $adminToken"}
    
    $user = $userResponse | Where-Object {$_.username -eq $username}
    
    if ($user) {
        # Create watch directory
        $body = @{ensure_created=$true} | ConvertTo-Json
        
        $result = Invoke-RestMethod `
            -Method Post `
            -Uri "$baseUrl/users/$($user.id)/watch-directory" `
            -Headers @{
                Authorization="Bearer $adminToken"
                "Content-Type"="application/json"
            } `
            -Body $body
        
        Write-Host "Created watch directory for $username at $($result.watch_directory_path)"
    }
}
```

### Best Practices Summary

#### For Administrators

1. **Capacity Planning**: Allocate 1-5GB per user for watch directories
2. **Backup Strategy**: Include user watch directories in backup plans
3. **Monitoring**: Set up alerts for disk space and processing failures
4. **Documentation**: Maintain user guide with network paths
5. **Testing**: Test with various file types and sizes before deployment

#### For Users

1. **File Organization**: Use meaningful filenames and folder structure
2. **File Formats**: Prefer PDF for best OCR results
3. **Batch Processing**: Group related documents for upload
4. **Size Limits**: Split large documents if over 50MB
5. **Patience**: Allow processing time before expecting search results

#### For Developers

1. **API Integration**: Use provided client libraries when available
2. **Error Handling**: Implement retry logic for transient failures
3. **Validation**: Validate file types before placing in watch directories
4. **Monitoring**: Track processing status via WebSocket updates
5. **Caching**: Cache user directory paths to reduce API calls

### Performance Optimization

1. **File System**: Use SSD storage for watch directories
2. **Network**: Minimize latency for network-mounted directories
3. **Scheduling**: Adjust watch interval based on usage patterns
4. **Concurrency**: Configure OCR workers based on CPU cores
5. **Cleanup**: Implement retention policies for processed files

### Migration from Global Watch Directory

To migrate from a single global watch directory to per-user directories:

1. **Preparation**:
   ```bash
   # Backup existing watch directory
   tar -czf watch_backup.tar.gz /data/watch/
   ```

2. **Enable Feature**:
   ```bash
   # Update configuration
   ENABLE_PER_USER_WATCH=true
   USER_WATCH_BASE_DIR=/data/user_watch
   ```

3. **Create User Directories**:
   ```bash
   # Script to create directories for existing users
   for user in $(psql -d readur -c "SELECT username FROM users" -t); do
     mkdir -p "/data/user_watch/$user"
     chown readur:readur "/data/user_watch/$user"
   done
   ```

4. **Migrate Documents** (optional):
   - Keep existing documents in place
   - Or reassign to appropriate users through the UI

5. **Update Documentation**:
   - Notify users of new directory locations
   - Update any automation scripts
   - Revise backup procedures

This completes the comprehensive documentation for the Per-User Watch Directories feature in Readur.