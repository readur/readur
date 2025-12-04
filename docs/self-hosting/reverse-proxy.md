# Reverse Proxy Configuration

## Overview

This guide covers configuring popular reverse proxies for Readur, including SSL/TLS setup, performance optimization, and security hardening.

## NGINX Configuration

### Basic HTTPS Setup

```nginx
# /etc/nginx/sites-available/readur
server {
    listen 80;
    server_name readur.company.com;
    
    # Redirect HTTP to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name readur.company.com;
    
    # SSL Configuration
    ssl_certificate /etc/ssl/certs/readur.crt;
    ssl_certificate_key /etc/ssl/private/readur.key;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;
    ssl_session_cache shared:SSL:10m;
    ssl_session_timeout 10m;
    
    # Security Headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;
    
    # Proxy Configuration
    location / {
        proxy_pass http://localhost:8000;
        proxy_http_version 1.1;
        
        # Headers
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header X-Forwarded-Host $host;
        proxy_set_header X-Forwarded-Port $server_port;
        
        # Timeouts
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
        
        # Buffering
        proxy_buffering off;
        proxy_request_buffering off;
    }
    
    # WebSocket support for real-time features
    location /ws {
        proxy_pass http://localhost:8000/ws;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
    
    # Large file uploads
    client_max_body_size 500M;
    client_body_timeout 600s;
}
```

### Performance Optimization

```nginx
# Enhanced configuration with caching and compression

upstream readur_backend {
    server localhost:8000 max_fails=3 fail_timeout=30s;
    server localhost:8001 backup;  # Optional backup server
    keepalive 32;
}

server {
    listen 443 ssl http2;
    server_name readur.company.com;
    
    # ... SSL configuration ...
    
    # Compression
    gzip on;
    gzip_vary on;
    gzip_min_length 1000;
    gzip_types text/plain text/css text/javascript 
               application/javascript application/json 
               application/xml image/svg+xml;
    
    # Static file caching
    location /static/ {
        alias /var/www/readur/static/;
        expires 30d;
        add_header Cache-Control "public, immutable";
        access_log off;
    }
    
    # Media files
    location /media/ {
        alias /var/www/readur/media/;
        expires 7d;
        add_header Cache-Control "public";
        
        # Security for uploaded files
        add_header Content-Disposition "attachment";
        add_header X-Content-Type-Options "nosniff";
    }
    
    # API endpoints
    location /api/ {
        proxy_pass http://readur_backend;
        
        # No caching for API
        proxy_no_cache 1;
        proxy_cache_bypass 1;
        
        # Rate limiting
        limit_req zone=api_limit burst=20 nodelay;
        limit_req_status 429;
    }
    
    # Main application
    location / {
        proxy_pass http://readur_backend;
        
        # ... proxy headers ...
        
        # Connection pooling
        proxy_http_version 1.1;
        proxy_set_header Connection "";
    }
}

# Rate limiting zones
limit_req_zone $binary_remote_addr zone=api_limit:10m rate=10r/s;
limit_req_zone $binary_remote_addr zone=upload_limit:10m rate=1r/s;
```

## Apache Configuration

### Basic Setup with mod_proxy

```apache
# /etc/apache2/sites-available/readur.conf
<VirtualHost *:80>
    ServerName readur.company.com
    
    # Redirect to HTTPS
    RewriteEngine On
    RewriteCond %{HTTPS} off
    RewriteRule ^(.*)$ https://%{HTTP_HOST}$1 [R=301,L]
</VirtualHost>

<VirtualHost *:443>
    ServerName readur.company.com
    
    # SSL Configuration
    SSLEngine on
    SSLCertificateFile /etc/ssl/certs/readur.crt
    SSLCertificateKeyFile /etc/ssl/private/readur.key
    SSLProtocol -all +TLSv1.2 +TLSv1.3
    SSLCipherSuite HIGH:!aNULL:!MD5:!3DES
    SSLHonorCipherOrder on
    
    # Security Headers
    Header always set Strict-Transport-Security "max-age=31536000"
    Header always set X-Frame-Options "SAMEORIGIN"
    Header always set X-Content-Type-Options "nosniff"
    Header always set X-XSS-Protection "1; mode=block"
    
    # Proxy Configuration
    ProxyPreserveHost On
    ProxyRequests Off
    
    ProxyPass / http://localhost:8000/
    ProxyPassReverse / http://localhost:8000/
    
    # WebSocket support
    RewriteEngine On
    RewriteCond %{HTTP:Upgrade} websocket [NC]
    RewriteCond %{HTTP:Connection} upgrade [NC]
    RewriteRule ^/?(.*) "ws://localhost:8000/$1" [P,L]
    
    # Request headers
    RequestHeader set X-Forwarded-Proto "https"
    RequestHeader set X-Forwarded-Port "443"
    
    # Timeouts
    ProxyTimeout 600
    
    # File upload size
    LimitRequestBody 524288000
</VirtualHost>
```

### Enable Required Modules

```bash
# Enable necessary Apache modules
sudo a2enmod proxy
sudo a2enmod proxy_http
sudo a2enmod proxy_wstunnel
sudo a2enmod headers
sudo a2enmod rewrite
sudo a2enmod ssl

# Enable site and reload
sudo a2ensite readur
sudo systemctl reload apache2
```

## Caddy Configuration

### Automatic HTTPS with Caddy

```caddy
# /etc/caddy/Caddyfile
readur.company.com {
    # Automatic HTTPS with Let's Encrypt
    
    # Reverse proxy
    reverse_proxy localhost:8000 {
        # Headers
        header_up Host {host}
        header_up X-Real-IP {remote}
        header_up X-Forwarded-For {remote}
        header_up X-Forwarded-Proto {scheme}
        
        # Health check
        health_uri /health
        health_interval 30s
        health_timeout 5s
        
        # Load balancing (if multiple backends)
        # lb_policy round_robin
        # lb_try_duration 5s
    }
    
    # File upload size
    request_body {
        max_size 500MB
    }
    
    # Timeouts
    timeouts {
        read 5m
        write 5m
        idle 10m
    }
    
    # Compression
    encode gzip zstd
    
    # Security headers
    header {
        Strict-Transport-Security "max-age=31536000; includeSubDomains"
        X-Frame-Options "SAMEORIGIN"
        X-Content-Type-Options "nosniff"
        X-XSS-Protection "1; mode=block"
        Referrer-Policy "strict-origin-when-cross-origin"
        -Server
    }
    
    # Static files
    handle_path /static/* {
        root * /var/www/readur/static
        file_server
        header Cache-Control "public, max-age=2592000"
    }
    
    # Rate limiting
    rate_limit {
        zone api {
            key {remote_host}
            events 100
            window 1m
        }
    }
    
    # Logging
    log {
        output file /var/log/caddy/readur.log {
            roll_size 100mb
            roll_keep 5
        }
        format json
    }
}
```

## Traefik Configuration

### Docker-based Setup

```yaml
# docker-compose.yml
version: '3.8'

services:
  traefik:
    image: traefik:v2.10
    command:
      - "--api.insecure=false"
      - "--providers.docker=true"
      - "--providers.docker.exposedbydefault=false"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.websecure.address=:443"
      - "--certificatesresolvers.letsencrypt.acme.tlschallenge=true"
      - "--certificatesresolvers.letsencrypt.acme.email=admin@company.com"
      - "--certificatesresolvers.letsencrypt.acme.storage=/letsencrypt/acme.json"
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
      - ./letsencrypt:/letsencrypt
    networks:
      - readur

  readur:
    image: ghcr.io/readur/readur:main
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.readur.rule=Host(`readur.company.com`)"
      - "traefik.http.routers.readur.entrypoints=websecure"
      - "traefik.http.routers.readur.tls.certresolver=letsencrypt"
      - "traefik.http.services.readur.loadbalancer.server.port=8000"
      # Middleware
      - "traefik.http.middlewares.readur-headers.headers.stsSeconds=31536000"
      - "traefik.http.middlewares.readur-headers.headers.stsIncludeSubdomains=true"
      - "traefik.http.middlewares.readur-headers.headers.frameDeny=true"
      - "traefik.http.middlewares.readur-headers.headers.contentTypeNosniff=true"
      - "traefik.http.middlewares.readur-ratelimit.ratelimit.average=100"
      - "traefik.http.routers.readur.middlewares=readur-headers,readur-ratelimit"
    networks:
      - readur

networks:
  readur:
    external: true
```

## HAProxy Configuration

### Load Balancing Setup

```haproxy
# /etc/haproxy/haproxy.cfg
global
    maxconn 4096
    log /dev/log local0
    log /dev/log local1 notice
    chroot /var/lib/haproxy
    user haproxy
    group haproxy
    daemon
    
    # SSL/TLS
    ssl-default-bind-ciphers ECDHE+AESGCM:ECDHE+AES256:!aNULL:!MD5:!DSS
    ssl-default-bind-options no-sslv3 no-tlsv10 no-tlsv11
    tune.ssl.default-dh-param 2048

defaults
    mode http
    log global
    option httplog
    option dontlognull
    option http-server-close
    option forwardfor except 127.0.0.0/8
    option redispatch
    retries 3
    timeout connect 5000ms
    timeout client 50000ms
    timeout server 50000ms
    
    # Compression
    compression algo gzip
    compression type text/html text/plain text/css text/javascript application/json

# Frontend
frontend readur_frontend
    bind *:80
    bind *:443 ssl crt /etc/ssl/readur.pem
    
    # Redirect HTTP to HTTPS
    redirect scheme https if !{ ssl_fc }
    
    # Security headers
    http-response set-header Strict-Transport-Security "max-age=31536000"
    http-response set-header X-Frame-Options "SAMEORIGIN"
    http-response set-header X-Content-Type-Options "nosniff"
    
    # ACLs
    acl is_websocket hdr(Upgrade) -i WebSocket
    acl is_api path_beg /api/
    acl is_static path_beg /static/
    
    # Rate limiting
    stick-table type ip size 100k expire 30s store http_req_rate(10s)
    http-request track-sc0 src
    http-request deny if { sc_http_req_rate(0) gt 100 }
    
    # Routing
    use_backend readur_websocket if is_websocket
    use_backend readur_static if is_static
    use_backend readur_api if is_api
    default_backend readur_app

# Backends
backend readur_app
    balance roundrobin
    option httpchk GET /health
    http-request set-header X-Forwarded-Port %[dst_port]
    http-request add-header X-Forwarded-Proto https if { ssl_fc }
    server app1 localhost:8000 check
    server app2 localhost:8001 check backup

backend readur_api
    balance leastconn
    server app1 localhost:8000 check
    server app2 localhost:8001 check backup

backend readur_static
    server static localhost:8080 check

backend readur_websocket
    server ws1 localhost:8000 check
```

## SSL/TLS Configuration

### Let's Encrypt with Certbot

```bash
# Install Certbot
sudo apt-get update
sudo apt-get install certbot python3-certbot-nginx

# Obtain certificate
sudo certbot --nginx -d readur.company.com

# Auto-renewal
sudo certbot renew --dry-run

# Cron job for renewal
echo "0 0,12 * * * root python -c 'import random; import time; time.sleep(random.random() * 3600)' && certbot renew -q" | sudo tee -a /etc/crontab > /dev/null
```

### Security Best Practices

```nginx
# Strong SSL configuration
ssl_protocols TLSv1.2 TLSv1.3;
ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384;
ssl_prefer_server_ciphers off;
ssl_session_timeout 1d;
ssl_session_cache shared:SSL:10m;
ssl_session_tickets off;
ssl_stapling on;
ssl_stapling_verify on;
ssl_trusted_certificate /etc/ssl/certs/chain.pem;

# HSTS
add_header Strict-Transport-Security "max-age=63072000; includeSubDomains; preload" always;
```

## Performance Tuning

### Connection Optimization

```nginx
# NGINX connection tuning
events {
    worker_connections 4096;
    use epoll;
    multi_accept on;
}

http {
    # Keepalive
    keepalive_timeout 65;
    keepalive_requests 100;
    
    # Buffers
    client_body_buffer_size 128k;
    client_header_buffer_size 1k;
    large_client_header_buffers 4 8k;
    output_buffers 32 32k;
    postpone_output 1460;
    
    # File handling
    sendfile on;
    tcp_nopush on;
    tcp_nodelay on;
}
```

### Caching Strategy

```nginx
# Cache configuration
proxy_cache_path /var/cache/nginx/readur 
    levels=1:2 
    keys_zone=readur_cache:100m 
    max_size=10g 
    inactive=60m
    use_temp_path=off;

location /api/documents/search {
    proxy_cache readur_cache;
    proxy_cache_valid 200 10m;
    proxy_cache_valid 404 1m;
    proxy_cache_key "$scheme$request_method$host$request_uri$args";
    proxy_cache_use_stale error timeout updating http_500 http_502 http_503 http_504;
    proxy_cache_background_update on;
    proxy_cache_lock on;
    add_header X-Cache-Status $upstream_cache_status;
}
```

## Monitoring

### Access Logs

```nginx
# Custom log format
log_format readur '$remote_addr - $remote_user [$time_local] '
                  '"$request" $status $body_bytes_sent '
                  '"$http_referer" "$http_user_agent" '
                  'rt=$request_time uct="$upstream_connect_time" '
                  'uht="$upstream_header_time" urt="$upstream_response_time"';

access_log /var/log/nginx/readur_access.log readur buffer=32k flush=5s;
error_log /var/log/nginx/readur_error.log warn;
```

### Health Checks

```nginx
# Health check endpoint
location /nginx-health {
    access_log off;
    add_header Content-Type text/plain;
    return 200 "healthy\n";
}
```

## Troubleshooting

### Common Issues

#### 502 Bad Gateway

```bash
# Check if Readur is running
curl -I http://localhost:8000/health

# Check logs
tail -f /var/log/nginx/error.log
docker-compose logs readur
```

#### 413 Request Entity Too Large

```nginx
# Increase upload limits
client_max_body_size 1G;
client_body_buffer_size 10M;
```

#### Slow Response Times

```bash
# Check upstream response time
tail -f /var/log/nginx/readur_access.log | grep "urt="

# Enable upstream keepalive
upstream readur_backend {
    server localhost:8000;
    keepalive 32;
}
```

## Related Documentation

- [SSL/TLS Best Practices](https://ssl-config.mozilla.org/)
- [NGINX Documentation](https://nginx.org/en/docs/)
- [Security Headers](https://securityheaders.com/)
- [Performance Tuning](./performance.md)