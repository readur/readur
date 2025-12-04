# Integration Guide

This guide covers integrating Readur with external systems, services, and tools to extend its functionality and fit into your existing infrastructure.

## Webhook Integration

### Configuring Webhooks

Readur can send webhooks for various events to integrate with external systems.

#### Setup

```yaml
# Environment configuration
WEBHOOK_ENABLED: true
WEBHOOK_URL: https://your-system.com/webhook
WEBHOOK_SECRET: your-webhook-secret
WEBHOOK_RETRY_ATTEMPTS: 3
WEBHOOK_TIMEOUT: 30
```

#### Webhook Events

| Event | Description | Payload |
|-------|-------------|---------|
| `document.created` | New document uploaded | Document details |
| `document.updated` | Document modified | Changes made |
| `document.deleted` | Document removed | Document ID |
| `ocr.completed` | OCR processing finished | OCR results |
| `ocr.failed` | OCR processing failed | Error details |
| `source.sync.started` | Source sync began | Source info |
| `source.sync.completed` | Source sync finished | Sync statistics |
| `user.created` | New user registered | User details |
| `user.login` | User logged in | Login info |

#### Webhook Payload Structure

```json
{
  "event": "document.created",
  "timestamp": "2025-01-15T10:30:00Z",
  "webhook_id": "whk_123456",
  "data": {
    "document_id": "doc_abc123",
    "title": "Invoice.pdf",
    "user_id": "usr_xyz789",
    "file_size": 1048576,
    "mime_type": "application/pdf"
  },
  "signature": "sha256=abcdef..."
}
```

#### Webhook Security

Verify webhook signatures:

```python
import hmac
import hashlib

def verify_webhook(payload, signature, secret):
    expected = hmac.new(
        secret.encode(),
        payload.encode(),
        hashlib.sha256
    ).hexdigest()
    
    return hmac.compare_digest(
        f"sha256={expected}",
        signature
    )

# Usage
@app.route('/webhook', methods=['POST'])
def handle_webhook():
    signature = request.headers.get('X-Readur-Signature')
    if not verify_webhook(request.data, signature, WEBHOOK_SECRET):
        return 'Unauthorized', 401
    
    # Process webhook
    data = request.json
    if data['event'] == 'document.created':
        process_new_document(data['data'])
    
    return 'OK', 200
```

### Webhook Examples

#### Slack Integration

```javascript
// Slack webhook handler
const express = require('express');
const axios = require('axios');

app.post('/readur-webhook', async (req, res) => {
  const event = req.body;
  
  if (event.event === 'ocr.completed') {
    await axios.post(process.env.SLACK_WEBHOOK_URL, {
      text: `Document processed: ${event.data.title}`,
      attachments: [{
        color: 'good',
        fields: [
          { title: 'Pages', value: event.data.pages, short: true },
          { title: 'Confidence', value: event.data.confidence, short: true }
        ]
      }]
    });
  }
  
  res.status(200).send('OK');
});
```

#### Zapier Integration

```python
# Zapier webhook transformer
def transform_for_zapier(readur_event):
    return {
        'id': readur_event['data']['document_id'],
        'title': readur_event['data']['title'],
        'created_at': readur_event['timestamp'],
        'file_url': f"https://readur.app/api/documents/{readur_event['data']['document_id']}/download",
        'metadata': readur_event['data'].get('metadata', {})
    }
```

## Storage Provider Integration

### S3-Compatible Storage

#### MinIO

```yaml
# MinIO configuration
S3_ENABLED: true
S3_ENDPOINT_URL: https://minio.internal:9000
S3_BUCKET_NAME: readur-documents
S3_ACCESS_KEY_ID: minioadmin
S3_SECRET_ACCESS_KEY: minioadmin
S3_USE_SSL: true
S3_VERIFY_SSL: false  # For self-signed certificates
```

#### Backblaze B2

```yaml
# Backblaze B2 configuration
S3_ENABLED: true
S3_ENDPOINT_URL: https://s3.us-west-002.backblazeb2.com
S3_BUCKET_NAME: your-bucket-name
S3_ACCESS_KEY_ID: your-key-id
S3_SECRET_ACCESS_KEY: your-application-key
```

#### DigitalOcean Spaces

```yaml
# DigitalOcean Spaces configuration
S3_ENABLED: true
S3_ENDPOINT_URL: https://nyc3.digitaloceanspaces.com
S3_BUCKET_NAME: your-space-name
S3_REGION: nyc3
S3_ACCESS_KEY_ID: your-access-key
S3_SECRET_ACCESS_KEY: your-secret-key
```

### Azure Blob Storage

```rust
// Future implementation example
pub struct AzureStorage {
    container_client: ContainerClient,
}

impl StorageBackend for AzureStorage {
    async fn store(&self, key: &str, data: &[u8]) -> Result<()> {
        let blob_client = self.container_client.blob_client(key);
        blob_client.put_block_blob(data).await?;
        Ok(())
    }
}
```

### Google Cloud Storage

```rust
// Future implementation example
pub struct GcsStorage {
    bucket: Bucket,
}

impl StorageBackend for GcsStorage {
    async fn store(&self, key: &str, data: &[u8]) -> Result<()> {
        self.bucket.create_object(key, data, "application/octet-stream").await?;
        Ok(())
    }
}
```

## Authentication Provider Integration

### OIDC/SSO Providers

#### Keycloak

```yaml
# Keycloak configuration
OIDC_ENABLED: true
OIDC_ISSUER_URL: https://keycloak.example.com/auth/realms/readur
OIDC_CLIENT_ID: readur-client
OIDC_CLIENT_SECRET: your-client-secret
OIDC_REDIRECT_URI: https://readur.example.com/api/auth/oidc/callback
OIDC_SCOPES: openid profile email
```

Keycloak client configuration:
```json
{
  "clientId": "readur-client",
  "standardFlowEnabled": true,
  "implicitFlowEnabled": false,
  "directAccessGrantsEnabled": false,
  "serviceAccountsEnabled": false,
  "publicClient": false,
  "frontchannelLogout": true,
  "protocol": "openid-connect",
  "redirectUris": [
    "https://readur.example.com/api/auth/oidc/callback"
  ],
  "webOrigins": [
    "https://readur.example.com"
  ]
}
```

#### Auth0

```yaml
# Auth0 configuration
OIDC_ENABLED: true
OIDC_ISSUER_URL: https://your-tenant.auth0.com/
OIDC_CLIENT_ID: your-client-id
OIDC_CLIENT_SECRET: your-client-secret
OIDC_REDIRECT_URI: https://readur.example.com/api/auth/oidc/callback
OIDC_SCOPES: openid profile email
```

#### Okta

```yaml
# Okta configuration
OIDC_ENABLED: true
OIDC_ISSUER_URL: https://your-org.okta.com/oauth2/default
OIDC_CLIENT_ID: your-client-id
OIDC_CLIENT_SECRET: your-client-secret
OIDC_REDIRECT_URI: https://readur.example.com/api/auth/oidc/callback
```

#### Azure AD

```yaml
# Azure AD configuration
OIDC_ENABLED: true
OIDC_ISSUER_URL: https://login.microsoftonline.com/{tenant-id}/v2.0
OIDC_CLIENT_ID: your-application-id
OIDC_CLIENT_SECRET: your-client-secret
OIDC_REDIRECT_URI: https://readur.example.com/api/auth/oidc/callback
OIDC_SCOPES: openid profile email User.Read
```

#### Google Workspace

```yaml
# Google Workspace configuration
OIDC_ENABLED: true
OIDC_ISSUER_URL: https://accounts.google.com
OIDC_CLIENT_ID: your-client-id.apps.googleusercontent.com
OIDC_CLIENT_SECRET: your-client-secret
OIDC_REDIRECT_URI: https://readur.example.com/api/auth/oidc/callback
OIDC_SCOPES: openid profile email
```

### LDAP Integration

```yaml
# Future LDAP support configuration
LDAP_ENABLED: true
LDAP_HOST: ldap.example.com
LDAP_PORT: 389
LDAP_USE_TLS: true
LDAP_BIND_DN: cn=readur,ou=services,dc=example,dc=com
LDAP_BIND_PASSWORD: password
LDAP_USER_BASE_DN: ou=users,dc=example,dc=com
LDAP_USER_FILTER: (uid={username})
LDAP_EMAIL_ATTRIBUTE: mail
LDAP_NAME_ATTRIBUTE: cn
```

## Monitoring Integration

### Prometheus

```yaml
# Enable Prometheus metrics
PROMETHEUS_ENABLED: true
PROMETHEUS_PORT: 9090
```

Prometheus configuration:
```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'readur'
    static_configs:
      - targets: ['readur:9090']
    metrics_path: '/metrics'
```

Key metrics to monitor:
```promql
# Document processing rate
rate(documents_processed_total[5m])

# OCR queue depth
ocr_queue_pending_count

# API response time
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))

# Error rate
rate(http_requests_total{status=~"5.."}[5m])
```

### Grafana

Import the Readur dashboard:

```json
{
  "dashboard": {
    "title": "Readur Monitoring",
    "panels": [
      {
        "title": "Documents Processed",
        "targets": [
          {
            "expr": "rate(documents_processed_total[5m])"
          }
        ]
      },
      {
        "title": "OCR Queue",
        "targets": [
          {
            "expr": "ocr_queue_pending_count"
          }
        ]
      },
      {
        "title": "API Latency",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))"
          }
        ]
      }
    ]
  }
}
```

### ELK Stack

#### Logstash Configuration

```ruby
# logstash.conf
input {
  tcp {
    port => 5000
    codec => json
  }
}

filter {
  if [app] == "readur" {
    grok {
      match => { "message" => "%{TIMESTAMP_ISO8601:timestamp} %{LOGLEVEL:level} %{GREEDYDATA:msg}" }
    }
    
    mutate {
      add_field => { "service" => "readur" }
    }
  }
}

output {
  elasticsearch {
    hosts => ["elasticsearch:9200"]
    index => "readur-%{+YYYY.MM.dd}"
  }
}
```

#### Filebeat Configuration

```yaml
# filebeat.yml
filebeat.inputs:
  - type: log
    enabled: true
    paths:
      - /var/log/readur/*.log
    json.keys_under_root: true
    json.add_error_key: true
    fields:
      service: readur

output.elasticsearch:
  hosts: ["elasticsearch:9200"]
  index: "readur-%{+yyyy.MM.dd}"
```

### Datadog

```yaml
# Datadog integration
DATADOG_ENABLED: true
DATADOG_API_KEY: your-api-key
DATADOG_APP_KEY: your-app-key
DATADOG_HOST: https://api.datadoghq.com
```

## Reverse Proxy Configuration

### Nginx

```nginx
# nginx.conf
upstream readur {
    server readur1:8080;
    server readur2:8080;
    server readur3:8080;
}

server {
    listen 443 ssl http2;
    server_name readur.example.com;
    
    ssl_certificate /etc/ssl/certs/readur.crt;
    ssl_certificate_key /etc/ssl/private/readur.key;
    
    # Security headers
    add_header Strict-Transport-Security "max-age=31536000" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    
    # Main application
    location / {
        proxy_pass http://readur;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
    
    # WebSocket support
    location /ws {
        proxy_pass http://readur;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_read_timeout 86400;
    }
    
    # File uploads
    location /api/documents/upload {
        proxy_pass http://readur;
        client_max_body_size 500M;
        proxy_request_buffering off;
    }
}
```

### Apache

```apache
# httpd.conf
<VirtualHost *:443>
    ServerName readur.example.com
    
    SSLEngine on
    SSLCertificateFile /etc/ssl/certs/readur.crt
    SSLCertificateKeyFile /etc/ssl/private/readur.key
    
    # Proxy configuration
    ProxyPreserveHost On
    ProxyPass / http://localhost:8080/
    ProxyPassReverse / http://localhost:8080/
    
    # WebSocket support
    RewriteEngine On
    RewriteCond %{HTTP:Upgrade} websocket [NC]
    RewriteCond %{HTTP:Connection} upgrade [NC]
    RewriteRule ^/?(.*) "ws://localhost:8080/$1" [P,L]
    
    # Security headers
    Header always set Strict-Transport-Security "max-age=31536000"
    Header always set X-Content-Type-Options "nosniff"
    Header always set X-Frame-Options "SAMEORIGIN"
</VirtualHost>
```

### Traefik

```yaml
# docker-compose.yml with Traefik
services:
  traefik:
    image: traefik:v2.9
    command:
      - "--providers.docker=true"
      - "--entrypoints.websecure.address=:443"
    ports:
      - "443:443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
      - ./certs:/certs
  
  readur:
    image: ghcr.io/readur/readur:main
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.readur.rule=Host(`readur.example.com`)"
      - "traefik.http.routers.readur.entrypoints=websecure"
      - "traefik.http.routers.readur.tls=true"
      - "traefik.http.services.readur.loadbalancer.server.port=8080"
```

### Caddy

```caddy
# Caddyfile
readur.example.com {
    reverse_proxy localhost:8080 {
        header_up X-Real-IP {remote_host}
        header_up X-Forwarded-Proto {scheme}
    }
    
    # File upload size
    request_body {
        max_size 500MB
    }
    
    # WebSocket support (automatic in Caddy)
}
```

## CI/CD Integration

### GitHub Actions

```yaml
# .github/workflows/readur-deploy.yml
name: Deploy Readur

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Build Docker image
        run: docker build -t readur:${{ github.sha }} .
      
      - name: Push to registry
        run: |
          echo ${{ secrets.DOCKER_PASSWORD }} | docker login -u ${{ secrets.DOCKER_USERNAME }} --password-stdin
          docker push readur:${{ github.sha }}
      
      - name: Deploy to Kubernetes
        uses: azure/k8s-deploy@v4
        with:
          manifests: |
            k8s/deployment.yaml
            k8s/service.yaml
          images: |
            readur:${{ github.sha }}
```

### GitLab CI

```yaml
# .gitlab-ci.yml
stages:
  - build
  - test
  - deploy

build:
  stage: build
  script:
    - docker build -t readur:$CI_COMMIT_SHA .
    - docker push $CI_REGISTRY_IMAGE:$CI_COMMIT_SHA

test:
  stage: test
  script:
    - cargo test
    - cargo clippy

deploy:
  stage: deploy
  script:
    - kubectl set image deployment/readur readur=$CI_REGISTRY_IMAGE:$CI_COMMIT_SHA
  only:
    - main
```

### Jenkins

```groovy
// Jenkinsfile
pipeline {
    agent any
    
    stages {
        stage('Build') {
            steps {
                sh 'docker build -t readur:${BUILD_NUMBER} .'
            }
        }
        
        stage('Test') {
            steps {
                sh 'cargo test'
            }
        }
        
        stage('Deploy') {
            when {
                branch 'main'
            }
            steps {
                sh 'kubectl apply -f k8s/'
                sh 'kubectl set image deployment/readur readur=readur:${BUILD_NUMBER}'
            }
        }
    }
}
```

## API Client Libraries

### Python SDK

```python
# readur-python-sdk
from readur import ReadurClient

client = ReadurClient(
    base_url="https://readur.example.com",
    api_key="your-api-key"
)

# Upload document
document = client.documents.upload(
    file_path="/path/to/document.pdf",
    metadata={"category": "invoice"}
)

# Search documents
results = client.search.query(
    q="invoice 2024",
    filters={"mime_type": "application/pdf"}
)

# WebSocket for real-time updates
@client.on('ocr.completed')
def handle_ocr_complete(event):
    print(f"OCR completed for {event['document_id']}")

client.connect_websocket()
```

### JavaScript/TypeScript SDK

```typescript
// @readur/sdk
import { ReadurClient } from '@readur/sdk';

const client = new ReadurClient({
  baseUrl: 'https://readur.example.com',
  apiKey: 'your-api-key'
});

// Upload document
const document = await client.documents.upload({
  file: fileInput.files[0],
  metadata: { category: 'invoice' }
});

// Search documents
const results = await client.search.query({
  q: 'invoice 2024',
  filters: { mimeType: 'application/pdf' }
});

// WebSocket subscription
client.subscribe('ocr.completed', (event) => {
  console.log(`OCR completed for ${event.documentId}`);
});
```

### Go SDK

```go
// github.com/readur/readur-go-sdk
package main

import (
    "github.com/readur/readur-go-sdk"
)

func main() {
    client := readur.NewClient(
        readur.WithBaseURL("https://readur.example.com"),
        readur.WithAPIKey("your-api-key"),
    )
    
    // Upload document
    doc, err := client.Documents.Upload(
        "document.pdf",
        readur.WithMetadata(map[string]interface{}{
            "category": "invoice",
        }),
    )
    
    // Search documents
    results, err := client.Search.Query(
        "invoice 2024",
        readur.WithFilter("mime_type", "application/pdf"),
    )
}
```

## Database Integration

### PostgreSQL Extensions

```sql
-- Enable useful extensions
CREATE EXTENSION IF NOT EXISTS pg_trgm;  -- Trigram similarity search
CREATE EXTENSION IF NOT EXISTS unaccent;  -- Remove accents
CREATE EXTENSION IF NOT EXISTS pgcrypto;  -- Encryption functions

-- Custom search function with fuzzy matching
CREATE OR REPLACE FUNCTION fuzzy_search(
    query_text TEXT,
    threshold FLOAT DEFAULT 0.3
) RETURNS TABLE (
    id UUID,
    title TEXT,
    similarity FLOAT
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        d.id,
        d.title,
        similarity(d.title, query_text) AS similarity
    FROM documents d
    WHERE similarity(d.title, query_text) > threshold
    ORDER BY similarity DESC;
END;
$$ LANGUAGE plpgsql;
```

### Redis Caching

```yaml
# Redis configuration
REDIS_ENABLED: true
REDIS_URL: redis://localhost:6379
REDIS_PASSWORD: your-password
REDIS_DB: 0
REDIS_KEY_PREFIX: readur:
```

Usage example:
```rust
// Cache search results
let cache_key = format!("search:{}", query_hash);
if let Some(cached) = redis.get(&cache_key).await? {
    return Ok(cached);
}

let results = perform_search(query).await?;
redis.set_ex(&cache_key, &results, 300).await?; // Cache for 5 minutes
```

## Notification Services

### Email (SMTP)

```yaml
# SMTP configuration
EMAIL_ENABLED: true
SMTP_HOST: smtp.gmail.com
SMTP_PORT: 587
SMTP_USERNAME: your-email@gmail.com
SMTP_PASSWORD: your-app-password
SMTP_FROM_ADDRESS: noreply@readur.app
SMTP_USE_TLS: true
```

### SendGrid

```yaml
# SendGrid configuration
EMAIL_PROVIDER: sendgrid
SENDGRID_API_KEY: your-api-key
SENDGRID_FROM_EMAIL: noreply@readur.app
SENDGRID_FROM_NAME: Readur
```

### Amazon SES

```yaml
# AWS SES configuration
EMAIL_PROVIDER: ses
AWS_REGION: us-east-1
S3_ACCESS_KEY_ID: your-access-key
S3_SECRET_ACCESS_KEY: your-secret-key
SES_FROM_EMAIL: noreply@readur.app
```

## Message Queue Integration

### RabbitMQ

```yaml
# RabbitMQ configuration
AMQP_ENABLED: true
AMQP_URL: amqp://user:pass@localhost:5672/
AMQP_EXCHANGE: readur-events
AMQP_QUEUE: readur-processing
```

### Apache Kafka

```yaml
# Kafka configuration
KAFKA_ENABLED: true
KAFKA_BROKERS: localhost:9092
KAFKA_TOPIC: readur-events
KAFKA_CONSUMER_GROUP: readur-consumers
```

## Best Practices

### Security

1. **Always use HTTPS** for webhooks and API calls
2. **Verify webhook signatures** to prevent spoofing
3. **Rotate API keys** regularly
4. **Use least privilege** for service accounts
5. **Enable audit logging** for all integrations

### Performance

1. **Implement retry logic** with exponential backoff
2. **Use connection pooling** for database connections
3. **Cache frequently accessed data**
4. **Batch API requests** when possible
5. **Monitor integration performance**

### Reliability

1. **Implement health checks** for all integrations
2. **Use circuit breakers** for external services
3. **Set appropriate timeouts**
4. **Handle failures gracefully**
5. **Maintain integration documentation**

## Troubleshooting Integrations

### Common Issues

#### Webhook Delivery Failures

```bash
# Check webhook logs
grep "webhook" /var/log/readur/app.log | tail -50

# Test webhook endpoint
curl -X POST https://your-webhook-url \
  -H "Content-Type: application/json" \
  -d '{"test": true}'
```

#### Authentication Failures

```bash
# Test OIDC discovery
curl https://your-provider/.well-known/openid-configuration

# Verify JWT token
jwt decode your-token-here
```

#### Storage Connection Issues

```bash
# Test S3 connectivity
aws s3 ls s3://your-bucket/ --endpoint-url https://your-endpoint

# Check permissions
aws s3api get-bucket-acl --bucket your-bucket
```