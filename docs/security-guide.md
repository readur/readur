# Security Best Practices Guide

This guide covers security configurations and best practices for deploying and operating Readur in production environments.

## Authentication and Authorization

### Authentication Setup

Readur supports multiple authentication methods to secure your document management system:

#### Local Authentication

```yaml
# Basic authentication configuration
AUTH_SECRET: "your-secure-random-secret-min-32-chars"
SESSION_SECRET: "your-session-secret-min-32-chars"
```

Generate secure secrets:
```bash
# Generate auth secret
openssl rand -hex 32

# Generate session secret
openssl rand -base64 32
```

#### OIDC/SSO Integration

Configure OpenID Connect for enterprise SSO:

```yaml
OIDC_ENABLED: "true"
OIDC_CLIENT_ID: "readur-client"
OIDC_CLIENT_SECRET: "your-client-secret"
OIDC_ISSUER_URL: "https://auth.example.com/realms/readur"
OIDC_REDIRECT_URI: "https://readur.example.com/api/auth/oidc/callback"
OIDC_SCOPES: "openid profile email"
```

### Role-Based Access Control

Readur implements three user roles with distinct permissions:

| Role | Permissions |
|------|------------|
| Admin | Full system access, user management, configuration changes |
| Editor | Document upload, edit, delete, OCR management |
| Viewer | Read-only access to documents and search |

Configure default role for new users:
```yaml
DEFAULT_USER_ROLE: "viewer"
AUTO_CREATE_USERS: "false"
```

### Session Management

Configure session security parameters:

```yaml
SESSION_TIMEOUT: 3600  # Seconds (1 hour)
SESSION_COOKIE_SECURE: "true"  # HTTPS only
SESSION_COOKIE_HTTPONLY: "true"  # Prevent XSS
SESSION_COOKIE_SAMESITE: "strict"  # CSRF protection
```

## File Upload Security

### Size Limits

Prevent resource exhaustion attacks:

```yaml
MAX_FILE_SIZE_MB: 100  # Maximum file size
MAX_FILES_PER_UPLOAD: 10  # Batch upload limit
TOTAL_STORAGE_QUOTA_GB: 1000  # Per-user quota
```

### File Type Validation

Restrict allowed file types:

```yaml
ALLOWED_FILE_TYPES: "pdf,png,jpg,jpeg,txt,doc,docx"
BLOCK_EXECUTABLE_FILES: "true"
SCAN_FOR_MALWARE: "true"  # Requires ClamAV integration
```

### Upload Validation

```rust
// Example validation implementation
fn validate_upload(file: &UploadedFile) -> Result<(), SecurityError> {
    // Check file size
    if file.size > MAX_FILE_SIZE {
        return Err(SecurityError::FileTooLarge);
    }
    
    // Validate MIME type
    let detected_mime = magic::from_buffer(&file.data)?;
    if !ALLOWED_MIME_TYPES.contains(&detected_mime) {
        return Err(SecurityError::InvalidFileType);
    }
    
    // Check for malicious content
    if contains_suspicious_patterns(&file.data) {
        return Err(SecurityError::MaliciousContent);
    }
    
    Ok(())
}
```

## Network Security

### TLS Configuration

Always use HTTPS in production:

```nginx
server {
    listen 443 ssl http2;
    ssl_certificate /etc/ssl/certs/readur.crt;
    ssl_certificate_key /etc/ssl/private/readur.key;
    
    # Modern TLS configuration
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256;
    ssl_prefer_server_ciphers off;
    
    # HSTS
    add_header Strict-Transport-Security "max-age=63072000" always;
}
```

### CORS Configuration

Configure Cross-Origin Resource Sharing:

```yaml
CORS_ENABLED: "true"
CORS_ALLOWED_ORIGINS: "https://app.example.com,https://admin.example.com"
CORS_ALLOWED_METHODS: "GET,POST,PUT,DELETE,OPTIONS"
CORS_MAX_AGE: 3600
```

### Rate Limiting

Prevent abuse and DoS attacks:

```yaml
RATE_LIMIT_ENABLED: "true"
RATE_LIMIT_REQUESTS_PER_MINUTE: 100
RATE_LIMIT_BURST_SIZE: 20
RATE_LIMIT_EXCLUDE_PATHS: "/health,/metrics"
```

## Secrets Management

### Environment Variables

Never commit secrets to version control:

```bash
# .env.example (commit this)
DATABASE_URL=postgresql://user:password@localhost/readur
AUTH_SECRET=change-this-secret
S3_ACCESS_KEY=your-access-key
S3_SECRET_KEY=your-secret-key

# .env (don't commit - add to .gitignore)
DATABASE_URL=postgresql://readur:SecurePass123!@db.internal/readur_prod
AUTH_SECRET=a8f7d9s8f7sd9f87sd9f87sd9f8s7df98s7df98s7df9
S3_ACCESS_KEY=AKIAIOSFODNN7EXAMPLE
S3_SECRET_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
```

### Secret Rotation

Implement regular secret rotation:

```bash
#!/bin/bash
# rotate-secrets.sh

# Generate new secrets
NEW_AUTH_SECRET=$(openssl rand -hex 32)
NEW_SESSION_SECRET=$(openssl rand -base64 32)

# Update application configuration
kubectl create secret generic readur-secrets \
  --from-literal=auth-secret="$NEW_AUTH_SECRET" \
  --from-literal=session-secret="$NEW_SESSION_SECRET" \
  --dry-run=client -o yaml | kubectl apply -f -

# Restart application
kubectl rollout restart deployment/readur
```

### Vault Integration

For production, use HashiCorp Vault or similar:

```yaml
VAULT_ENABLED: "true"
VAULT_ADDR: "https://vault.internal:8200"
VAULT_TOKEN: "s.xxxxxxxxxxxxxxxx"
VAULT_PATH: "secret/data/readur"
```

## Data Encryption

### Encryption at Rest

Database encryption:

```sql
-- PostgreSQL transparent data encryption
ALTER SYSTEM SET ssl = on;
ALTER SYSTEM SET ssl_cert_file = '/etc/postgresql/server.crt';
ALTER SYSTEM SET ssl_key_file = '/etc/postgresql/server.key';
```

S3 storage encryption:

```yaml
S3_ENCRYPTION_ENABLED: "true"
S3_ENCRYPTION_TYPE: "AES256"  # or "aws:kms"
S3_KMS_KEY_ID: "arn:aws:kms:region:account:key/xxxxx"
```

### Encryption in Transit

All data transmissions must be encrypted:

```yaml
# Database connections
DATABASE_SSL_MODE: "require"
DATABASE_SSL_CERT: "/etc/ssl/certs/db-cert.pem"

# S3 connections
S3_USE_SSL: "true"
S3_VERIFY_SSL: "true"

# Redis connections (if used)
REDIS_TLS_ENABLED: "true"
REDIS_TLS_CERT: "/etc/ssl/certs/redis-cert.pem"
```

## Audit Logging

### Comprehensive Audit Trail

Configure audit logging for security events:

```yaml
AUDIT_LOG_ENABLED: "true"
AUDIT_LOG_LEVEL: "info"
AUDIT_LOG_PATH: "/var/log/readur/audit.log"
AUDIT_LOG_FORMAT: "json"
```

Audit log entry structure:
```json
{
  "timestamp": "2025-01-15T10:30:45Z",
  "event_type": "document_access",
  "user_id": "user123",
  "user_email": "user@example.com",
  "action": "download",
  "resource": "document/abc123",
  "ip_address": "192.168.1.100",
  "user_agent": "Mozilla/5.0...",
  "result": "success",
  "metadata": {
    "document_name": "financial_report.pdf",
    "file_size": 1048576
  }
}
```

### Events to Audit

Critical events that must be logged:
- User authentication (success/failure)
- User registration
- Password changes
- Document uploads/downloads
- Document deletions
- Permission changes
- Configuration changes
- Admin actions
- Failed authorization attempts
- Suspicious activities

## Common Vulnerabilities and Mitigations

### SQL Injection Prevention

Always use parameterized queries:

```rust
// Safe query using sqlx
let document = sqlx::query_as!(
    Document,
    "SELECT * FROM documents WHERE id = $1 AND user_id = $2",
    document_id,
    user_id
)
.fetch_one(&pool)
.await?;

// Never do this!
// let query = format!("SELECT * FROM documents WHERE id = {}", id);
```

### XSS Prevention

Sanitize all user input:

```rust
use ammonia::clean;

fn sanitize_input(input: &str) -> String {
    // Remove potentially dangerous HTML
    clean(input)
}

// Content Security Policy headers
app.use(
    DefaultHeaders::new()
        .header("Content-Security-Policy", "default-src 'self'")
        .header("X-Content-Type-Options", "nosniff")
        .header("X-Frame-Options", "DENY")
        .header("X-XSS-Protection", "1; mode=block")
);
```

### CSRF Protection

Implement CSRF tokens:

```rust
// Generate CSRF token
let csrf_token = generate_csrf_token(&session);

// Validate on form submission
if !validate_csrf_token(&request.csrf_token, &session) {
    return Err(SecurityError::InvalidCSRFToken);
}
```

### Directory Traversal Prevention

Validate file paths:

```rust
use std::path::{Path, Component};

fn safe_path(user_input: &str) -> Result<PathBuf, SecurityError> {
    let path = Path::new(user_input);
    
    // Check for directory traversal attempts
    for component in path.components() {
        match component {
            Component::ParentDir => return Err(SecurityError::PathTraversal),
            Component::RootDir => return Err(SecurityError::AbsolutePath),
            _ => {}
        }
    }
    
    Ok(path.to_path_buf())
}
```

## Security Headers

Configure security headers in your reverse proxy:

```nginx
# Security headers
add_header X-Content-Type-Options "nosniff" always;
add_header X-Frame-Options "SAMEORIGIN" always;
add_header X-XSS-Protection "1; mode=block" always;
add_header Referrer-Policy "strict-origin-when-cross-origin" always;
add_header Content-Security-Policy "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline';" always;
add_header Permissions-Policy "geolocation=(), microphone=(), camera=()" always;
```

## Security Checklist

### Pre-Deployment Checklist

- [ ] All secrets are properly managed (not in code)
- [ ] TLS/SSL certificates are valid and configured
- [ ] Authentication is properly configured
- [ ] File upload restrictions are in place
- [ ] Rate limiting is enabled
- [ ] CORS is properly configured
- [ ] Security headers are set
- [ ] Audit logging is enabled
- [ ] Database connections use SSL
- [ ] Default passwords are changed
- [ ] Unnecessary ports are closed
- [ ] Error messages don't leak sensitive information

### Regular Security Tasks

- [ ] Review audit logs weekly
- [ ] Update dependencies monthly
- [ ] Rotate secrets quarterly
- [ ] Conduct security assessments annually
- [ ] Test backup restoration procedures
- [ ] Review user permissions
- [ ] Monitor for suspicious activities
- [ ] Update TLS certificates before expiry

## Security Monitoring

### Key Metrics to Monitor

```yaml
# Prometheus alerts
groups:
  - name: security
    rules:
      - alert: HighFailedLoginRate
        expr: rate(auth_failures_total[5m]) > 10
        annotations:
          summary: "High rate of failed login attempts"
      
      - alert: UnusualFileUploadVolume
        expr: rate(file_uploads_total[1h]) > 100
        annotations:
          summary: "Unusual file upload activity detected"
      
      - alert: SuspiciousAPIUsage
        expr: rate(api_requests_total{status="403"}[5m]) > 20
        annotations:
          summary: "High rate of forbidden API requests"
```

## Incident Response

### Security Incident Procedure

1. **Detection**: Identify the security incident
2. **Containment**: Isolate affected systems
3. **Investigation**: Determine scope and impact
4. **Eradication**: Remove the threat
5. **Recovery**: Restore normal operations
6. **Lessons Learned**: Document and improve

### Emergency Contacts

Maintain an updated contact list:
- Security team lead
- System administrators
- Database administrators
- Cloud provider support
- Legal counsel (if needed)

## Additional Resources

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [CIS Security Benchmarks](https://www.cisecurity.org/cis-benchmarks/)
- [NIST Cybersecurity Framework](https://www.nist.gov/cyberframework)
- [PostgreSQL Security Documentation](https://www.postgresql.org/docs/current/security.html)