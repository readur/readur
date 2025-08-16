# Sources Guide

Readur's Sources feature provides powerful automated document ingestion from multiple external storage systems. This comprehensive guide covers all supported source types and their configuration.

## Table of Contents

- [Overview](#overview)
- [Source Types](#source-types)
  - [WebDAV Sources](#webdav-sources)
  - [Local Folder Sources](#local-folder-sources)
  - [S3 Sources](#s3-sources)
- [Getting Started](#getting-started)
- [Configuration](#configuration)
- [Sync Operations](#sync-operations)
- [Health Monitoring](#health-monitoring)
- [Troubleshooting](#troubleshooting)
- [Best Practices](#best-practices)

## Overview

Sources allow Readur to automatically discover, download, and process documents from external storage systems. Key features include:

- **Multi-Protocol Support**: WebDAV, Local Folders, and S3-compatible storage
- **Automated Syncing**: Scheduled synchronization with configurable intervals
- **Health Monitoring**: Proactive monitoring and validation of source connections
- **Intelligent Processing**: Duplicate detection, incremental syncs, and OCR integration
- **Real-time Status**: Live sync progress via WebSocket connections
- **Per-User Watch Directories**: Individual watch folders for each user (v2.5.4+)

### How Sources Work

1. **Configuration**: Set up a source with connection details and preferences
2. **Discovery**: Readur scans the source for supported file types
3. **Synchronization**: New and changed files are downloaded and processed
4. **OCR Processing**: Documents are automatically queued for text extraction
5. **Search Integration**: Processed documents become searchable in your collection

## Source Types

### WebDAV Sources

WebDAV sources connect to cloud storage services and self-hosted servers that support the WebDAV protocol.

#### Supported WebDAV Servers

| Server Type | Status | Notes |
|-------------|--------|-------|
| **Nextcloud** | ✅ Fully Supported | Optimized discovery and authentication |
| **ownCloud** | ✅ Fully Supported | Native integration with server detection |
| **Apache WebDAV** | ✅ Supported | Generic WebDAV implementation |
| **nginx WebDAV** | ✅ Supported | Works with nginx dav module |
| **Box.com** | ⚠️ Limited | Basic WebDAV support |
| **Other WebDAV** | ✅ Supported | Generic WebDAV protocol compliance |

#### WebDAV Configuration

**Required Fields:**
- **Name**: Descriptive name for the source
- **Server URL**: Full WebDAV server URL (e.g., `https://cloud.example.com/remote.php/dav/files/username/`)
- **Username**: WebDAV authentication username
- **Password**: WebDAV authentication password or app password

**Optional Configuration:**
- **Watch Folders**: Specific directories to monitor (leave empty to sync entire accessible space)
- **File Extensions**: Limit to specific file types (default: all supported types)
- **Auto Sync**: Enable automatic scheduled synchronization
- **Sync Interval**: How often to check for changes (15 minutes to 24 hours)
- **Server Type**: Specify server type for optimizations (auto-detected)

#### Setting Up WebDAV Sources

1. **Navigate to Sources**: Go to Settings → Sources in the Readur interface
2. **Add New Source**: Click "Add Source" and select "WebDAV"
3. **Configure Connection**:
   ```
   Name: My Nextcloud Documents
   Server URL: https://cloud.mycompany.com/remote.php/dav/files/john/
   Username: john
   Password: app-password-here
   ```
4. **Test Connection**: Use the "Test Connection" button to verify credentials
5. **Configure Folders**: Specify directories to monitor:
   ```
   Watch Folders:
   - Documents/
   - Projects/2024/
   - Invoices/
   ```
6. **Set Sync Schedule**: Choose automatic sync interval (recommended: 30 minutes)
7. **Save and Sync**: Save configuration and trigger initial sync

#### WebDAV Best Practices

- **Use App Passwords**: Create dedicated app passwords instead of using main account passwords
- **Limit Scope**: Specify watch folders to avoid syncing unnecessary files
- **Server Optimization**: Let Readur auto-detect server type for optimal performance
- **Network Considerations**: Use longer sync intervals for slow connections

### Local Folder Sources

Local folder sources monitor directories on the Readur server's filesystem, including mounted network drives.

#### Use Cases

- **Watch Folders**: Monitor directories where documents are dropped
- **Network Mounts**: Sync from NFS, SMB/CIFS, or other mounted filesystems
- **Batch Processing**: Automatically process documents placed in specific folders
- **Archive Integration**: Monitor existing document archives
- **Per-User Ingestion**: Individual watch directories for each user (v2.5.4+)

#### Local Folder Configuration

**Required Fields:**
- **Name**: Descriptive name for the source
- **Watch Folders**: Absolute paths to monitor directories

**Optional Configuration:**
- **File Extensions**: Filter by specific file types
- **Auto Sync**: Enable scheduled monitoring
- **Sync Interval**: Frequency of directory scans
- **Recursive**: Include subdirectories in scans
- **Follow Symlinks**: Follow symbolic links (use with caution)

#### Setting Up Local Folder Sources

1. **Prepare Directory**: Ensure the directory exists and is accessible
   ```bash
   # Create watch folder
   mkdir -p /mnt/documents/inbox
   
   # Set permissions (if needed)
   chmod 755 /mnt/documents/inbox
   ```

2. **Configure Source**:
   ```
   Name: Document Inbox
   Watch Folders: /mnt/documents/inbox
   File Extensions: pdf,jpg,png,txt,docx
   Auto Sync: Enabled
   Sync Interval: 5 minutes
   Recursive: Yes
   ```

3. **Test Setup**: Place a test document in the folder and verify detection

#### Network Mount Examples

**NFS Mount:**
```bash
# Mount NFS share
sudo mount -t nfs 192.168.1.100:/documents /mnt/nfs-docs

# Configure in Readur
Watch Folders: /mnt/nfs-docs/inbox
```

**SMB/CIFS Mount:**
```bash
# Mount SMB share
sudo mount -t cifs //server/documents /mnt/smb-docs -o username=user

# Configure in Readur
Watch Folders: /mnt/smb-docs/processing
```

#### Per-User Watch Directories (v2.5.4+)

Each user can have their own dedicated watch directory for automatic document ingestion. This feature is ideal for multi-tenant deployments, department separation, and maintaining clear data boundaries.

**Configuration:**
```bash
# Enable per-user watch directories
ENABLE_PER_USER_WATCH=true
USER_WATCH_BASE_DIR=/data/user_watches
```

**Directory Structure:**
```
/data/user_watches/
├── john_doe/
│   ├── invoice.pdf
│   └── report.docx
├── jane_smith/
│   └── presentation.pptx
└── admin/
    └── policy.pdf
```

**API Management:**
```http
# Get user watch directory info
GET /api/users/{userId}/watch-directory

# Create/ensure watch directory exists
POST /api/users/{userId}/watch-directory
{
  "ensure_created": true
}

# Delete user watch directory
DELETE /api/users/{userId}/watch-directory
```

**Use Cases:**
- **Multi-tenant deployments**: Isolate document ingestion per customer
- **Department separation**: Each department has its own ingestion folder
- **Compliance**: Maintain clear data separation between users
- **Automation**: Connect scanners or automation tools to user-specific folders

### S3 Sources

S3 sources connect to Amazon S3 or S3-compatible storage services for document synchronization.

> 📖 **Complete S3 Guide**: For detailed S3 storage backend configuration, migration from local storage, and advanced features, see the [S3 Storage Guide](s3-storage-guide.md).

#### Supported S3 Services

| Service | Status | Configuration |
|---------|--------|---------------|
| **Amazon S3** | ✅ Fully Supported | Standard AWS configuration |
| **MinIO** | ✅ Fully Supported | Custom endpoint URL |
| **DigitalOcean Spaces** | ✅ Supported | S3-compatible API |
| **Wasabi** | ✅ Supported | Custom endpoint configuration |
| **Google Cloud Storage** | ⚠️ Limited | S3-compatible mode only |

#### S3 Configuration

**Required Fields:**
- **Name**: Descriptive name for the source
- **Bucket Name**: S3 bucket to monitor
- **Region**: AWS region (e.g., `us-east-1`)
- **Access Key ID**: AWS/S3 access key
- **Secret Access Key**: AWS/S3 secret key

**Optional Configuration:**
- **Endpoint URL**: Custom endpoint for S3-compatible services
- **Prefix**: Bucket path prefix to limit scope
- **Watch Folders**: Specific S3 "directories" to monitor
- **File Extensions**: Filter by file types
- **Auto Sync**: Enable scheduled synchronization
- **Sync Interval**: Frequency of bucket scans

#### Setting Up S3 Sources

1. **Prepare S3 Bucket**: Ensure bucket exists and credentials have access
2. **Configure Source**:
   ```
   Name: Company Documents S3
   Bucket Name: company-documents
   Region: us-west-2
   Access Key ID: AKIAIOSFODNN7EXAMPLE
   Secret Access Key: wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
   Prefix: documents/
   Watch Folders: 
   - invoices/
   - contracts/
   - reports/
   ```

3. **Test Connection**: Verify credentials and bucket access

#### S3-Compatible Services

**MinIO Configuration:**
```
Endpoint URL: https://minio.example.com:9000
Bucket Name: documents
Region: us-east-1  (can be any value for MinIO)
```

**DigitalOcean Spaces:**
```
Endpoint URL: https://nyc3.digitaloceanspaces.com
Bucket Name: my-documents
Region: nyc3
```

## Getting Started

### Adding Your First Source

1. **Access Sources Management**: Navigate to Settings → Sources
2. **Choose Source Type**: Select WebDAV, Local Folder, or S3 based on your needs
3. **Configure Connection**: Enter required credentials and connection details
4. **Test Connection**: Verify connectivity before saving
5. **Configure Sync**: Set up folders to monitor and sync schedule
6. **Initial Sync**: Trigger first synchronization to import existing documents

### Quick Setup Examples

#### Nextcloud WebDAV
```
Name: Nextcloud Documents
Server URL: https://cloud.company.com/remote.php/dav/files/username/
Username: username
Password: app-password
Watch Folders: Documents/, Shared/
Auto Sync: Every 30 minutes
```

#### Local Network Drive
```
Name: Network Archive
Watch Folders: /mnt/network/documents
File Extensions: pdf,doc,docx,txt
Recursive: Yes
Auto Sync: Every 15 minutes
```

#### AWS S3 Bucket
```
Name: AWS Document Bucket
Bucket: company-docs-bucket
Region: us-east-1
Access Key: [AWS Access Key]
Secret Key: [AWS Secret Key]
Prefix: active-documents/
Auto Sync: Every 1 hour
```

## Configuration

### Sync Settings

**Sync Intervals:**
- **Real-time**: Immediate processing (local folders only)
- **5-15 minutes**: High-frequency monitoring
- **30-60 minutes**: Standard monitoring (recommended)
- **2-24 hours**: Low-frequency, large dataset sync

**File Filtering:**
- **File Extensions**: `pdf,jpg,jpeg,png,txt,doc,docx,rtf`
- **Size Limits**: Configurable maximum file size (default: 50MB)
- **Path Exclusions**: Skip specific directories or file patterns

### Advanced Configuration

**Concurrency Settings:**
- **Concurrent Files**: Number of files processed simultaneously (default: 5)
- **Network Timeout**: Connection timeout for network sources
- **Retry Logic**: Automatic retry for failed downloads

**Deduplication:**
- **Hash-based**: SHA-256 content hashing prevents duplicate storage
- **Cross-source**: Duplicates detected across all sources
- **Metadata Preservation**: Tracks file origins while avoiding storage duplication

## Sync Operations

### Manual Sync

**Trigger Immediate Sync:**
1. Navigate to Sources page
2. Find the source to sync
3. Click the "Sync Now" button
4. Monitor progress in real-time

**Deep Scan:**
- Forces complete re-scan of entire source
- Useful for detecting changes in large directories
- Automatically triggered periodically

### Sync Status

**Status Indicators:**
- 🟢 **Idle**: Source ready, no sync in progress
- 🟡 **Syncing**: Active synchronization in progress
- 🔴 **Error**: Sync failed, requires attention
- ⚪ **Disabled**: Source disabled, no automatic sync

**Progress Information:**
- Files discovered vs. processed
- Current operation (scanning, downloading, processing)
- Estimated completion time
- Transfer speeds and statistics

### Real-Time Sync Progress (v2.5.4+)

Readur uses WebSocket connections for real-time sync progress updates, providing lower latency and bidirectional communication compared to the previous Server-Sent Events implementation.

**WebSocket Connection:**
```javascript
// Connect to sync progress WebSocket
const ws = new WebSocket('wss://readur.example.com/api/sources/{sourceId}/sync/progress');

ws.onmessage = (event) => {
  const progress = JSON.parse(event.data);
  console.log(`Sync progress: ${progress.percentage}%`);
};
```

**Progress Event Format:**
```json
{
  "phase": "discovering",
  "progress": 45,
  "current_file": "document.pdf",
  "total_files": 150,
  "processed_files": 68,
  "status": "in_progress"
}
```

**Benefits:**
- Bidirectional communication for interactive control
- 50% reduction in bandwidth compared to SSE
- Automatic reconnection handling
- Lower server CPU usage

### Stopping Sync

**Graceful Cancellation:**
1. Click "Stop Sync" button during active sync
2. Current file processing completes
3. Sync stops cleanly without corruption
4. Partial progress is saved

## Health Monitoring

### Health Scores

Sources are continuously monitored and assigned health scores (0-100):

- **90-100**: ✅ Excellent - No issues detected
- **75-89**: ⚠️ Good - Minor issues or warnings
- **50-74**: ⚠️ Fair - Moderate issues requiring attention
- **25-49**: ❌ Poor - Significant problems
- **0-24**: ❌ Critical - Severe issues, manual intervention required

### Health Checks

**Automatic Validation** (every 30 minutes):
- Connection testing
- Credential verification
- Configuration validation
- Sync pattern analysis
- Error rate monitoring

**Common Health Issues:**
- Authentication failures
- Network connectivity problems
- Permission or access issues
- Configuration errors
- Rate limiting or throttling

### Health Notifications

**Alert Types:**
- Connection failures
- Authentication expires
- Sync errors
- Performance degradation
- Configuration warnings

## Troubleshooting

### Common Issues

#### WebDAV Connection Problems

**Symptom**: "Connection failed" or authentication errors
**Solutions**:
1. Verify server URL format:
   - Nextcloud: `https://server.com/remote.php/dav/files/username/`
   - ownCloud: `https://server.com/remote.php/dav/files/username/`
   - Generic: `https://server.com/webdav/`

2. Check credentials:
   - Use app passwords instead of main passwords
   - Verify username/password combination
   - Test credentials in web browser or WebDAV client

3. Network issues:
   - Verify server is accessible from Readur
   - Check firewall and SSL certificate issues
   - Test with curl: `curl -u username:password https://server.com/webdav/`

#### Local Folder Issues

**Symptom**: "Permission denied" or "Directory not found"
**Solutions**:
1. Check directory permissions:
   ```bash
   ls -la /path/to/watch/folder
   chmod 755 /path/to/watch/folder  # If needed
   ```

2. Verify path exists:
   ```bash
   stat /path/to/watch/folder
   ```

3. For network mounts:
   ```bash
   mount | grep /path/to/mount  # Verify mount
   ls -la /path/to/mount        # Test access
   ```

#### S3 Access Problems

**Symptom**: "Access denied" or "Bucket not found"
**Solutions**:
1. Verify credentials and permissions:
   ```bash
   aws s3 ls s3://bucket-name --profile your-profile
   ```

2. Check bucket policy and IAM permissions
3. Verify region configuration matches bucket region
4. For S3-compatible services, ensure correct endpoint URL

### Performance Issues

#### Slow Sync Performance

**Causes and Solutions**:
1. **Large file sizes**: Increase timeout values, consider file size limits
2. **Network latency**: Reduce concurrent connections, increase intervals
3. **Server throttling**: Implement longer delays between requests
4. **Large directories**: Use watch folders to limit scope

#### High Resource Usage

**Optimization Strategies**:
1. **Reduce concurrency**: Lower concurrent file processing
2. **Increase intervals**: Less frequent sync checks
3. **Filter files**: Limit to specific file types and sizes
4. **Stagger syncs**: Avoid multiple sources syncing simultaneously

### Error Recovery

**Automatic Recovery:**
- Failed files are automatically retried
- Temporary network issues are handled gracefully
- Sync resumes from last successful point

**Manual Recovery:**
1. Check source health status
2. Review error logs in source details
3. Test connection manually
4. Trigger deep scan to reset sync state

## Best Practices

### Security

1. **Use Dedicated Credentials**: Create app-specific passwords and access keys
2. **Limit Permissions**: Grant minimum required access to source accounts
3. **Regular Rotation**: Periodically update passwords and access keys
4. **Network Security**: Use HTTPS/TLS for all connections

### Performance

1. **Strategic Scheduling**: Stagger sync times for multiple sources
2. **Scope Limitation**: Use watch folders to limit sync scope
3. **File Filtering**: Exclude unnecessary file types and large files
4. **Monitor Resources**: Watch CPU, memory, and network usage

### Organization

1. **Descriptive Names**: Use clear, descriptive source names
2. **Consistent Structure**: Maintain consistent folder organization
3. **Documentation**: Document source purposes and configurations
4. **Regular Maintenance**: Periodically review and clean up sources

### Reliability

1. **Health Monitoring**: Regularly check source health scores
2. **Backup Configuration**: Document source configurations
3. **Test Scenarios**: Periodically test sync and recovery procedures
4. **Monitor Logs**: Review sync logs for patterns or issues

## Next Steps

- Configure [notifications](notifications-guide.md) for sync events
- Set up [advanced search](advanced-search.md) to find synced documents
- Review [OCR optimization](dev/OCR_OPTIMIZATION_GUIDE.md) for processing improvements
- Explore [labels and organization](labels-and-organization.md) for document management