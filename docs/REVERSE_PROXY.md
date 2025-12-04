# Reverse Proxy Configuration Guide for Readur

This guide covers various deployment scenarios for running Readur behind reverse proxies with configurable ports.

## Table of Contents
- [Port Configuration](#port-configuration)
- [Docker Deployment](#docker-deployment)
- [Nginx Configuration](#nginx-configuration)
- [Traefik Configuration](#traefik-configuration)
- [Apache Configuration](#apache-configuration)
- [Caddy Configuration](#caddy-configuration)
- [Common Scenarios](#common-scenarios)
- [Troubleshooting](#troubleshooting)

## Port Configuration

Readur supports flexible port configuration through environment variables:

### Server Port Configuration

```bash
# Method 1: Full address specification
SERVER_ADDRESS=0.0.0.0:3000

# Method 2: Separate host and port (recommended)
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
```

### Development Port Configuration

For frontend development with Vite:

```bash
# Backend API port
BACKEND_PORT=8000

# Frontend dev server port
CLIENT_PORT=5173
```

## Docker Deployment

### Basic Docker Run

```bash
# Run on custom port 3000
docker run -d \
  -e SERVER_PORT=3000 \
  -e DATABASE_URL=postgresql://user:pass@host/db \
  -e JWT_SECRET=your-secret-key \
  -p 3000:3000 \
  readur:latest
```

### Docker Compose with Custom Ports

```yaml
version: '3.8'

services:
  readur:
    image: ghcr.io/readur/readur:main
    environment:
      SERVER_PORT: 3000
      DATABASE_URL: postgresql://readur:readur@postgres/readur
      JWT_SECRET: ${JWT_SECRET}
    ports:
      - "3000:3000"
```

## Nginx Configuration

### Basic Reverse Proxy

```nginx
server {
    listen 80;
    server_name readur.example.com;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # Large file uploads for documents
        client_max_body_size 100M;
        
        # Extended timeouts for OCR processing
        proxy_connect_timeout 300s;
        proxy_send_timeout 300s;
        proxy_read_timeout 300s;
    }
}
```

### SSL Configuration

```nginx
server {
    listen 443 ssl http2;
    server_name readur.example.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto https;
        
        # WebSocket support (future feature)
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

### Subpath Configuration

To serve Readur under a subpath like `/readur/`:

```nginx
location /readur/ {
    # Remove prefix when passing to backend
    rewrite ^/readur/(.*) /$1 break;
    
    proxy_pass http://localhost:3000;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_set_header X-Forwarded-Prefix /readur;
}
```

## Traefik Configuration

### Docker Labels

```yaml
version: '3.8'

services:
  readur:
    image: ghcr.io/readur/readur:main
    environment:
      SERVER_PORT: 3000
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.readur.rule=Host(`readur.example.com`)"
      - "traefik.http.routers.readur.tls=true"
      - "traefik.http.routers.readur.tls.certresolver=letsencrypt"
      - "traefik.http.services.readur.loadbalancer.server.port=3000"
      # Middleware for large uploads
      - "traefik.http.middlewares.readur-upload.buffering.maxRequestBodyBytes=104857600"
      - "traefik.http.routers.readur.middlewares=readur-upload"
```

### Static Configuration

```yaml
# traefik.yml
http:
  routers:
    readur:
      rule: "Host(`readur.example.com`)"
      service: readur
      tls:
        certResolver: letsencrypt
      middlewares:
        - readur-headers

  services:
    readur:
      loadBalancer:
        servers:
          - url: "http://localhost:3000"
        healthCheck:
          path: /api/health
          interval: 30s

  middlewares:
    readur-headers:
      headers:
        customRequestHeaders:
          X-Forwarded-Proto: https
```

## Apache Configuration

### Basic Reverse Proxy

```apache
<VirtualHost *:80>
    ServerName readur.example.com
    
    ProxyRequests Off
    ProxyPreserveHost On
    
    ProxyPass / http://localhost:3000/
    ProxyPassReverse / http://localhost:3000/
    
    # Large file uploads
    LimitRequestBody 104857600
    
    # Extended timeouts
    ProxyTimeout 300
</VirtualHost>
```

### SSL Configuration

```apache
<VirtualHost *:443>
    ServerName readur.example.com
    
    SSLEngine on
    SSLCertificateFile /path/to/cert.pem
    SSLCertificateKeyFile /path/to/key.pem
    
    ProxyRequests Off
    ProxyPreserveHost On
    
    ProxyPass / http://localhost:3000/
    ProxyPassReverse / http://localhost:3000/
    
    RequestHeader set X-Forwarded-Proto "https"
</VirtualHost>
```

## Caddy Configuration

### Caddyfile

```caddy
readur.example.com {
    reverse_proxy localhost:3000 {
        header_up X-Real-IP {remote_host}
        header_up X-Forwarded-Proto {scheme}
        
        # Health check
        health_uri /api/health
        health_interval 30s
        
        # Extended timeouts
        transport http {
            dial_timeout 10s
            response_header_timeout 300s
        }
    }
    
    # Large file uploads
    request_body {
        max_size 100MB
    }
}
```

## Common Scenarios

### Multiple Instances with Load Balancing

#### Nginx
```nginx
upstream readur_pool {
    least_conn;
    server readur1:3000 max_fails=3 fail_timeout=30s;
    server readur:3000 max_fails=3 fail_timeout=30s;
    server readur3:3000 max_fails=3 fail_timeout=30s;
}

server {
    location / {
        proxy_pass http://readur_pool;
        # ... other proxy settings
    }
}
```

#### Docker Compose Scale
```bash
# Scale to 3 instances
docker compose up -d --scale readur=3
```

### Blue-Green Deployment

```nginx
# nginx.conf
upstream readur_blue {
    server blue:3000;
}

upstream readur_green {
    server green:3000;
}

# Switch between blue and green
upstream readur_current {
    server blue:3000;  # Change to green:3000 for switch
}

server {
    location / {
        proxy_pass http://readur_current;
    }
}
```

### Rate Limiting

```nginx
# Define rate limit zones
limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;
limit_req_zone $binary_remote_addr zone=auth:10m rate=5r/m;

server {
    # Apply to API endpoints
    location /api/ {
        limit_req zone=api burst=20 nodelay;
        proxy_pass http://localhost:3000;
    }
    
    # Stricter for auth
    location /api/auth/ {
        limit_req zone=auth burst=5 nodelay;
        proxy_pass http://localhost:3000;
    }
}
```

## Troubleshooting

### Common Issues

1. **502 Bad Gateway**
   - Check if Readur is running on the configured port
   - Verify `SERVER_PORT` environment variable
   - Check container logs: `docker logs readur`

2. **Large File Upload Failures**
   - Increase `client_max_body_size` in nginx
   - Adjust `MAX_FILE_SIZE_MB` in Readur config
   - Check proxy timeout settings

3. **WebSocket Connection Issues** (future feature)
   - Ensure `proxy_http_version 1.1` is set
   - Include Upgrade and Connection headers

4. **CORS Errors**
   - Readur includes CORS middleware
   - Ensure `X-Forwarded-Proto` header is set correctly
   - Check if frontend URL matches expected origin

### Health Check Monitoring

```bash
# Direct health check
curl http://localhost:3000/api/health

# Through reverse proxy
curl https://readur.example.com/api/health

# Expected response
{"status":"ok"}
```

### Debug Headers

Add these to your reverse proxy for debugging:

```nginx
add_header X-Proxy-Debug "nginx" always;
add_header X-Upstream-Addr $upstream_addr always;
add_header X-Upstream-Status $upstream_status always;
```

## Security Recommendations

1. **Always use HTTPS** in production
2. **Implement rate limiting** to prevent abuse
3. **Set security headers**:
   ```nginx
   add_header X-Frame-Options "SAMEORIGIN" always;
   add_header X-Content-Type-Options "nosniff" always;
   add_header X-XSS-Protection "1; mode=block" always;
   add_header Referrer-Policy "strict-origin-when-cross-origin" always;
   ```

4. **Restrict access** to health check endpoint if needed:
   ```nginx
   location /api/health {
       allow 10.0.0.0/8;
       deny all;
       proxy_pass http://localhost:3000;
   }
   ```

5. **Use strong JWT secrets** and rotate them regularly
6. **Monitor access logs** for suspicious activity

## Performance Optimization

1. **Enable caching** for static assets:
   ```nginx
   location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2)$ {
       proxy_pass http://localhost:3000;
       expires 1y;
       add_header Cache-Control "public, immutable";
   }
   ```

2. **Use HTTP/2** for better performance
3. **Enable gzip compression**:
   ```nginx
   gzip on;
   gzip_types text/plain text/css application/json application/javascript;
   gzip_min_length 1000;
   ```

4. **Configure connection pooling** for database
5. **Use CDN** for static assets in production