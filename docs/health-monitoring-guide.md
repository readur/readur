# 📊 Health Monitoring Guide

Readur includes comprehensive health monitoring to ensure system reliability and proactive issue detection.

## Overview

The health monitoring system continuously validates:
- Data source connectivity and status
- System resource utilization
- Processing queue health
- Database performance
- OCR engine availability

## Monitoring Dashboard

Access health information through:
- **Admin Panel** → Health Status
- **API Endpoints** for programmatic monitoring
- **Real-time Alerts** for immediate issue notification

## Source Health Validation

### WebDAV Sources
- Connection testing every 5 minutes
- Authentication validation
- Network latency monitoring
- Error rate tracking

### Local Folder Sources
- Directory accessibility checks
- Permission validation
- Disk space monitoring
- File system health

### S3-Compatible Sources
- Bucket accessibility
- Credential validation
- Region connectivity
- API rate limit monitoring

## System Health Metrics

### Performance Indicators

- **CPU Usage**  
  System load monitoring
  
- **Memory Usage**  
  RAM utilization tracking
  
- **Disk Space**  
  Storage capacity alerts
  
- **Queue Length**  
  Processing backlog size

### Processing Health

- **OCR Success Rate**  
  Text extraction reliability
  
- **Processing Speed**  
  Documents per minute
  
- **Error Rates**  
  Failed operation tracking
  
- **Retry Attempts**  
  Automatic recovery metrics

## Alert Configuration

### Alert Types

- **Critical**  
  System failures requiring immediate attention
  
- **Warning**  
  Performance degradation or resource limits
  
- **Info**  
  Status updates and maintenance notifications

### Notification Methods

- **In-App Notifications**  
  Real-time dashboard alerts
  
- **Email Alerts**  
  Configurable email notifications
  
- **Webhook Integration**  
  External system notifications

## Health Check Endpoints

### API Health Checks
```bash
# Liveness: answers {"status": "ok"} without authentication
GET /api/health

# OCR engine and language data (any signed-in user)
GET /api/ocr/health

# Database, OCR queue, document and user figures (administrators only)
GET /api/metrics

# Health check of one source's configuration and connectivity
POST /api/sources/{id}/validate
```

See the [API Reference](api-reference.md) for the response formats.

## Troubleshooting

### Common Issues

- **Source Disconnected**  
  Check network connectivity and credentials
  
- **High Queue Length**  
  Scale processing resources or optimize OCR
  
- **Memory Warnings**  
  Review document processing batch sizes
  
- **Disk Space Low**  
  Clean up temporary files or expand storage

### Recovery Actions

- **Automatic Retry**  
  Failed operations retry with exponential backoff
  
- **Graceful Degradation**  
  System continues operating with reduced functionality
  
- **Manual Intervention**  
  Admin tools for resolving complex issues

## Configuration

Health monitoring can be configured in your environment:

```env
# Health check intervals (seconds)
HEALTH_CHECK_INTERVAL=300
SOURCE_CHECK_INTERVAL=600

# Alert thresholds
CPU_WARNING_THRESHOLD=80
MEMORY_WARNING_THRESHOLD=85
DISK_WARNING_THRESHOLD=90

# Notification settings
HEALTH_EMAIL_ALERTS=true
WEBHOOK_URL=https://your-monitoring-system.com/webhook
```