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

Sources transform Readur from a simple upload-based system into a comprehensive document management platform that automatically stays synchronized with your existing storage. Instead of manually uploading documents, you connect Readur to where your documents already live, and it handles discovery, processing, and indexing automatically.

The Sources feature supports multiple storage protocols including WebDAV (for cloud services like Nextcloud), local folders and network mounts, and S3-compatible storage. Synchronization runs on schedules you configure, with built-in health monitoring to alert you when connections have issues. The system intelligently handles duplicate detection across sources and integrates seamlessly with OCR processing.

Real-time status updates keep you informed of sync progress through WebSocket connections, providing immediate feedback when large batches are processing. The latest version adds per-user watch directories, allowing individual users to have their own dedicated document ingestion folders.

### How the Source System Works

When you configure a source, you provide connection details and specify which folders or storage areas to monitor. Readur then scans these locations periodically to discover new or changed files that match supported document types. During synchronization, any new or modified files are downloaded and queued for processing.

The OCR system automatically processes documents as they arrive, extracting text and making them searchable. Once processing completes, documents become immediately available through Readur's search interface, with all the same capabilities as manually uploaded files.

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

Setting up a WebDAV source requires several key pieces of information. You'll need a descriptive name to identify the source in your Readur interface, and the complete WebDAV server URL. The URL format varies by provider - Nextcloud and ownCloud typically use paths like `https://cloud.example.com/remote.php/dav/files/username/`, while generic WebDAV servers might use simpler paths.

Authentication requires your WebDAV username and password. For security, many cloud providers let you generate app-specific passwords rather than using your main account password - this approach is strongly recommended since it allows you to revoke access to Readur without affecting other applications.

For monitoring scope, you can specify particular directories to watch, or leave this empty to sync all accessible files. File extension filtering lets you limit synchronization to specific document types if you want to avoid processing certain files. The sync interval determines how frequently Readur checks for changes - more frequent intervals provide faster updates but use more server resources.

Readur can often auto-detect your server type (Nextcloud, ownCloud, etc.) to optimize connection handling, but you can specify this manually if auto-detection doesn't work correctly.

#### Setting Up WebDAV Sources

To create a WebDAV source, start by navigating to Settings → Sources in the Readur interface, then click "Add Source" and select "WebDAV" from the available options. In the configuration form, provide connection details like this example for a Nextcloud server:

```
Name: My Nextcloud Documents
Server URL: https://cloud.mycompany.com/remote.php/dav/files/john/
Username: john
Password: app-password-here
```

Always use the "Test Connection" button to verify your credentials work before proceeding. This test confirms that Readur can authenticate and access your WebDAV server successfully.

Next, configure which directories to monitor by specifying watch folders. You might set up monitoring like this:

```
Watch Folders:
- Documents/
- Projects/2024/
- Invoices/
```

Choose an appropriate sync schedule - 30 minutes is a good balance between staying current and not overwhelming your server with requests. Finally, save your configuration and trigger an initial sync to import existing documents.

#### WebDAV Best Practices

For security, always create dedicated app passwords in your cloud provider rather than using your main account password. This practice lets you revoke Readur's access independently if needed. Limit the scope of synchronization by specifying watch folders rather than syncing your entire cloud storage - this avoids processing personal files or unrelated documents.

Let Readur auto-detect your server type when possible, as this enables optimizations specific to your WebDAV implementation. For slow or unreliable network connections, use longer sync intervals to reduce the chance of timeout errors during synchronization.

### Local Folder Sources

Local folder sources monitor directories on the Readur server's filesystem, including mounted network drives.

#### Common Use Cases for Local Folders

Local folder sources work well for traditional file server environments where documents are regularly deposited into specific directories. Watch folders provide automatic processing for documents dropped by scanners, email systems, or other automated processes. Network mounts let you sync from shared storage systems using protocols like NFS or SMB/CIFS without requiring additional credentials.

Batch processing scenarios benefit from local sources when you need to process large numbers of documents placed in staging directories. Archive integration helps bring existing document collections under Readur's search capabilities by monitoring established document repositories. The latest version also supports per-user ingestion directories, giving each user their own dedicated drop folder.

#### Configuring Local Folder Sources

Setting up a local folder source requires a descriptive name and the absolute paths to directories you want to monitor. The paths must be accessible from the Readur server and should use complete filesystem paths rather than relative references.

You can filter monitoring to specific file types using file extension lists, which helps avoid processing irrelevant files in mixed-use directories. Enable automatic sync with appropriate intervals based on how frequently documents arrive - frequent arrivals might warrant 5-minute intervals, while archive monitoring might only need hourly checks.

The recursive option includes subdirectories in monitoring, which is useful for hierarchical document structures. Use the symlink following option cautiously, as it can lead to infinite loops if symbolic links create circular references in your filesystem.

#### Setting Up Local Folder Sources

Before configuring a local folder source in Readur, ensure the target directory exists and has appropriate permissions. Create the directory structure and set permissions that allow the Readur process to read files:

```bash
# Create the watch folder structure
mkdir -p /mnt/documents/inbox

# Set appropriate permissions for Readur access
chmod 755 /mnt/documents/inbox
```

In the Readur interface, configure your source with settings appropriate for your use case:

```
Name: Document Inbox
Watch Folders: /mnt/documents/inbox
File Extensions: pdf,jpg,png,txt,docx
Auto Sync: Enabled
Sync Interval: 5 minutes
Recursive: Yes
```

This configuration monitors the inbox directory every 5 minutes, processes common document types, and includes any subdirectories. After saving the configuration, test your setup by placing a document in the watched folder and confirming that Readur detects and processes it.

#### Working with Network Mounts

Network-mounted storage expands local folder capabilities to include remote file systems. For NFS shares, mount the remote filesystem and then configure Readur to monitor specific paths within it:

```bash
# Mount an NFS share to your local filesystem
sudo mount -t nfs 192.168.1.100:/documents /mnt/nfs-docs

# Configure Readur to watch a specific path in the mounted share
Watch Folders: /mnt/nfs-docs/inbox
```

SMB/CIFS shares work similarly, requiring mount commands with appropriate credentials:

```bash
# Mount SMB share with authentication
sudo mount -t cifs //server/documents /mnt/smb-docs -o username=user

# Point Readur to a processing directory within the mount
Watch Folders: /mnt/smb-docs/processing
```

Ensure network mounts are stable and available when Readur starts, as missing mounts can cause source health issues.

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

#### S3 Configuration Requirements

Setting up an S3 source requires several essential pieces of information. You'll need a descriptive name for the source, the exact name of the S3 bucket to monitor, and the AWS region where the bucket is located (such as `us-east-1` or `us-west-2`). Authentication requires both an access key ID and secret access key with appropriate permissions to list and download objects from the bucket.

For S3-compatible services that aren't Amazon S3, you'll also need to specify a custom endpoint URL. The prefix option lets you limit monitoring to a specific "directory" within the bucket, which is useful for large buckets with mixed content. Watch folders provide additional filtering to monitor only specific paths within your chosen prefix.

File extension filtering and sync intervals work the same as other source types, letting you control which files are processed and how frequently Readur checks for changes.

#### Setting Up S3 Sources

Before configuring the source in Readur, ensure your S3 bucket exists and your access credentials have the necessary permissions. Your IAM user or role should be able to list bucket contents and download objects from the monitored paths.

Configure your S3 source with details appropriate for your storage setup:

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

This configuration monitors specific "directories" within the documents prefix of your bucket. Always use the "Test Connection" button to verify that your credentials work and Readur can access the specified bucket before saving the configuration.

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

Creating your first source begins with accessing the Sources management interface by navigating to Settings → Sources in Readur. From there, choose the source type that matches your storage system - WebDAV for cloud services like Nextcloud, Local Folder for server directories or network mounts, or S3 for cloud object storage.

Enter the required connection details and credentials for your chosen source type. Each source type has different requirements, but they generally include authentication information and the location of your documents. Always use the "Test Connection" feature to verify that Readur can successfully connect to your storage before proceeding.

Configure which folders or areas to monitor and set an appropriate sync schedule based on how frequently your documents change. Finally, trigger an initial synchronization to import existing documents from your source. This first sync might take a while depending on how many documents are already stored in your source location.

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

- **90-100**: ✅ Excellent  
  No issues detected
  
- **75-89**: ⚠️ Good  
  Minor issues or warnings
  
- **50-74**: ⚠️ Fair  
  Moderate issues requiring attention
  
- **25-49**: ❌ Poor  
  Significant problems
  
- **0-24**: ❌ Critical  
  Severe issues, manual intervention required

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