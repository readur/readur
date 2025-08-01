# Command-Line Tools Reference

## Overview

Readur includes several command-line utilities for system administration and maintenance. These tools are designed for system administrators and DevOps teams managing Readur deployments.

## migrate_to_s3

**Purpose:** Migrate document storage between backends (Local ↔ S3)

### Usage
```bash
migrate_to_s3 [OPTIONS]
```

### Command Options

| Option | Description | Example |
|--------|-------------|---------|
| `--dry-run` | Test migration without making changes | `--dry-run` |
| `--enable-rollback` | Enable rollback capabilities with state tracking | `--enable-rollback` |
| `--user-id <UUID>` | Migrate documents for specific user only | `--user-id "123e4567-..."` |
| `--resume-from <FILE>` | Resume migration from saved state file | `--resume-from /tmp/state.json` |
| `--rollback <FILE>` | Rollback previous migration using state file | `--rollback /tmp/state.json` |
| `--batch-size <NUM>` | Number of documents to process per batch | `--batch-size 1000` |
| `--parallel-uploads <NUM>` | Maximum concurrent S3 uploads | `--parallel-uploads 5` |
| `--verbose` | Enable detailed output and progress logging | `--verbose` |
| `--audit-files` | Check file system consistency before migration | `--audit-files` |
| `--status` | Show status of current/recent migrations | `--status` |
| `--help` | Display help information | `--help` |

### Examples

#### Basic Migration
```bash
# Test migration first
docker exec readur-app cargo run --bin migrate_to_s3 -- --dry-run

# Run actual migration with safety features
docker exec readur-app cargo run --bin migrate_to_s3 -- --enable-rollback

# Verbose migration with custom batch size
docker exec readur-app cargo run --bin migrate_to_s3 -- \
  --enable-rollback --verbose --batch-size 500
```

#### User-Specific Migration
```bash
# Get user IDs from database
docker exec readur-app psql -d readur -c \
  "SELECT id, email FROM users WHERE email LIKE '%@company.com';"

# Migrate specific user's documents
docker exec readur-app cargo run --bin migrate_to_s3 -- \
  --enable-rollback --user-id "uuid-from-above"
```

#### Recovery Operations
```bash
# Resume interrupted migration
docker exec readur-app cargo run --bin migrate_to_s3 -- \
  --resume-from /tmp/migration_state_20241201_143022.json

# Rollback completed migration
docker exec readur-app cargo run --bin migrate_to_s3 -- \
  --rollback /tmp/migration_state_20241201_143022.json

# Check migration status
docker exec readur-app cargo run --bin migrate_to_s3 -- --status
```

#### Performance Optimization
```bash
# High-performance migration for large datasets
docker exec readur-app cargo run --bin migrate_to_s3 -- \
  --enable-rollback \
  --batch-size 2000 \
  --parallel-uploads 10 \
  --verbose

# Conservative migration for limited resources
docker exec readur-app cargo run --bin migrate_to_s3 -- \
  --enable-rollback \
  --batch-size 100 \
  --parallel-uploads 2
```

### State Files

The migration tool creates state files to track progress and enable recovery:

**Location:** `/tmp/migration_state_YYYYMMDD_HHMMSS.json`

**Contents:**
```json
{
  "migration_id": "uuid",
  "started_at": "2024-12-01T14:30:22Z",
  "completed_migrations": [
    {
      "document_id": "uuid",
      "original_path": "/app/uploads/doc.pdf",
      "s3_key": "documents/user123/doc.pdf",
      "migrated_at": "2024-12-01T14:31:15Z"
    }
  ],
  "failed_migrations": [],
  "total_files": 2500,
  "processed_files": 1247,
  "rollback_enabled": true
}
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Configuration error |
| 3 | Database connection error |
| 4 | S3 access error |
| 5 | File system error |
| 10 | Migration already in progress |
| 11 | State file not found |
| 12 | Rollback failed |

## enqueue_pending_ocr

**Purpose:** Add documents with pending OCR status to the processing queue

### Usage
```bash
docker exec readur-app cargo run --bin enqueue_pending_ocr
```

### Description
This utility addresses situations where documents are marked as pending OCR in the database but haven't been added to the OCR processing queue. This can happen after:
- Database restoration
- System crashes during OCR processing
- Migration from older versions

### Example Output
```
🔍 Scanning for documents with pending OCR status...
📊 Found 45 documents with pending OCR status
🚀 Enqueuing documents for OCR processing...
✅ Successfully enqueued 45 documents
⏱️  Average queue priority: 5
📈 Current queue size: 127 items
```

### When to Use
- After restoring from database backup
- When OCR queue appears empty but documents show "pending" status
- Following system recovery or migration
- As part of maintenance procedures

## test_runner

**Purpose:** Execute comprehensive test suites with detailed reporting

### Usage
```bash
docker exec readur-app cargo run --bin test_runner [OPTIONS]
```

### Options
| Option | Description |
|--------|-------------|
| `--unit` | Run unit tests only |
| `--integration` | Run integration tests only |
| `--e2e` | Run end-to-end tests only |
| `--verbose` | Detailed test output |
| `--parallel <N>` | Number of parallel test threads |

### Examples
```bash
# Run all tests
docker exec readur-app cargo run --bin test_runner

# Run only integration tests with verbose output
docker exec readur-app cargo run --bin test_runner -- --integration --verbose

# Run tests with limited parallelism
docker exec readur-app cargo run --bin test_runner -- --parallel 2
```

## General Usage Patterns

### Docker Deployments
For Docker-based Readur deployments:

```bash
# General pattern
docker exec readur-app cargo run --bin <tool-name> -- [OPTIONS]

# With environment variables
docker exec -e S3_BUCKET_NAME=my-bucket readur-app \
  cargo run --bin migrate_to_s3 -- --dry-run

# Interactive mode (if needed)
docker exec -it readur-app cargo run --bin migrate_to_s3 -- --help
```

### Direct Deployments
For direct server deployments:

```bash
# Ensure proper working directory
cd /path/to/readur

# Run with production environment
RUST_ENV=production ./target/release/migrate_to_s3 --dry-run

# With custom configuration
DATABASE_URL="postgresql://..." ./target/release/migrate_to_s3 --status
```

### Kubernetes Deployments
For Kubernetes environments:

```bash
# Find the pod name
kubectl get pods -l app=readur

# Execute tool in pod
kubectl exec deployment/readur -- \
  cargo run --bin migrate_to_s3 -- --dry-run

# With environment variable override
kubectl exec deployment/readur -e S3_REGION=eu-west-1 -- \
  cargo run --bin migrate_to_s3 -- --status
```

## Best Practices

### Before Running Tools
1. **Backup data** - Always backup database and files
2. **Test in staging** - Try commands in non-production first
3. **Check resources** - Ensure sufficient CPU, memory, disk space
4. **Verify access** - Confirm database and S3 connectivity

### During Execution
1. **Monitor progress** - Watch logs and system resources
2. **Keep sessions active** - Use `screen` or `tmux` for long operations
3. **Save output** - Redirect output to files for later analysis
4. **Document actions** - Keep notes of commands and results

### After Completion
1. **Verify results** - Check that operations completed successfully
2. **Clean up** - Remove temporary files and state data if appropriate
3. **Update documentation** - Record any configuration changes
4. **Monitor application** - Watch for any issues after changes

## Environment Variables

Common environment variables used by CLI tools:

| Variable | Purpose | Example |
|----------|---------|---------|
| `DATABASE_URL` | PostgreSQL connection string | `postgresql://user:pass@host:5432/readur` |
| `S3_BUCKET_NAME` | Target S3 bucket | `my-company-readur` |
| `S3_ACCESS_KEY_ID` | AWS access key | `AKIA...` |
| `S3_SECRET_ACCESS_KEY` | AWS secret key | `...` |
| `S3_REGION` | AWS region | `us-east-1` |
| `S3_ENDPOINT` | Custom S3 endpoint | `https://minio.company.com` |
| `RUST_LOG` | Logging level | `debug`, `info`, `warn`, `error` |
| `RUST_BACKTRACE` | Error backtraces | `1` or `full` |

## Troubleshooting

### Common Issues

1. **Permission Denied**
   ```bash
   # Check container user
   docker exec readur-app whoami
   
   # Fix file permissions if needed
   docker exec readur-app chown -R readur:readur /app/uploads
   ```

2. **Tool Not Found**
   ```bash
   # List available binaries
   docker exec readur-app find target/release -name "*migrate*" -type f
   
   # Build tools if missing
   docker exec readur-app cargo build --release --bins
   ```

3. **Database Connection Issues**
   ```bash
   # Test database connectivity
   docker exec readur-app psql -d readur -c "SELECT version();"
   
   # Check environment variables
   docker exec readur-app env | grep DATABASE_URL
   ```

### Getting Help

For each tool, use the `--help` flag:
```bash
docker exec readur-app cargo run --bin migrate_to_s3 -- --help
docker exec readur-app cargo run --bin enqueue_pending_ocr -- --help
```

### Logging and Debugging

Enable detailed logging:
```bash
# Debug level logging
docker exec -e RUST_LOG=debug readur-app \
  cargo run --bin migrate_to_s3 -- --verbose

# With backtrace for errors
docker exec -e RUST_BACKTRACE=1 readur-app \
  cargo run --bin migrate_to_s3 -- --status
```

## Security Considerations

### Access Control
- CLI tools should only be run by system administrators
- Use proper Docker user contexts
- Limit access to state files containing sensitive information

### Credential Handling
- Never log full credentials or API keys
- Use environment variables instead of command-line parameters
- Rotate credentials after major operations

### Network Security
- Ensure TLS/HTTPS for all S3 communications
- Use VPN or private networks when possible
- Monitor network traffic during migrations

Remember: These tools have significant impact on your Readur deployment. Always test in non-production environments first and maintain proper backups.