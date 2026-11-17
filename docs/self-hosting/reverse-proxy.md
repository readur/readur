# Reverse Proxy Configuration

## Overview

Readur serves its web interface and API from one port (8000 by default) over plain HTTP. In production, put a reverse proxy in front of it to terminate HTTPS. This page has configurations for NGINX, Apache, Caddy, Traefik and HAProxy; [Reverse Proxy Configuration Guide](../REVERSE_PROXY.md) covers custom ports and more scenarios.

Whatever proxy you use, it needs to:

- forward every path (the web interface, its assets and the API) to Readur; there are no separate static or media directories to serve;
- accept request bodies at least as large as `MAX_FILE_SIZE_MB` (50 MB by default), the largest upload Readur accepts;
- pass WebSocket upgrades for `/api/sources/{id}/sync/progress/ws`, which streams source sync progress to the web interface;
- send `X-Forwarded-For` so Readur's login rate limiting sees the real client address.

## Readur Settings for a Proxy

| Variable | Purpose |
|----------|---------|
| `PUBLIC_URL` | The public base URL, e.g. `https://readur.example.com`. Used for the OIDC post-login redirect and shared-link URLs (request headers such as `Host` are not used for this). When it starts with `https://`, Readur also sends `Strict-Transport-Security: max-age=31536000` |
| `TRUSTED_PROXIES` | IPs or CIDRs of your proxies, comma-separated. `X-Forwarded-For` is only used for requests that arrive from these addresses. When unset, loopback and private networks are trusted, which covers a proxy on the same host or Docker network; `none` trusts no proxy |
| `SERVER_HOST`, `SERVER_PORT` | Where Readur listens (default `0.0.0.0:8000`). Set `SERVER_HOST=127.0.0.1` to accept connections from a proxy on the same host only |
| `CORS_ALLOWED_ORIGINS` | Not needed: the web interface is served from the same origin. Only set it when a different origin must call the API |

```bash
PUBLIC_URL=https://readur.example.com
TRUSTED_PROXIES=127.0.0.1
```

Readur already adds `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`, `Content-Security-Policy` and `Cross-Origin-Opener-Policy` to its responses (see [Security Headers](./authentication.md#security-headers)), so the proxy does not need to add them. Readur does not compress responses itself, so enabling compression in the proxy is worthwhile.

Do not cache API responses in the proxy: they depend on who is signed in.

## NGINX

```nginx
# /etc/nginx/conf.d/readur.conf
map $http_upgrade $connection_upgrade {
    default upgrade;
    ''      close;
}

server {
    listen 80;
    server_name readur.example.com;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl http2;
    server_name readur.example.com;

    ssl_certificate     /etc/letsencrypt/live/readur.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/readur.example.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;

    # At least MAX_FILE_SIZE_MB
    client_max_body_size 100M;

    gzip on;
    gzip_types text/plain text/css application/javascript application/json image/svg+xml;

    location / {
        proxy_pass http://127.0.0.1:8000;
        proxy_http_version 1.1;

        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # WebSocket (sync progress)
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $connection_upgrade;

        # Large uploads and downloads
        proxy_read_timeout 300s;
        proxy_send_timeout 300s;
    }
}
```

The repository's `docker-compose.yml` also has an optional `nginx` service (profile `with-proxy`) that mounts `./nginx.conf` and `./ssl`; start it with `docker compose --profile with-proxy up -d` after writing that configuration. Inside that Compose network, proxy to `http://readur:8000`.

## Apache

```apache
# /etc/apache2/sites-available/readur.conf
<VirtualHost *:80>
    ServerName readur.example.com
    Redirect permanent / https://readur.example.com/
</VirtualHost>

<VirtualHost *:443>
    ServerName readur.example.com

    SSLEngine on
    SSLCertificateFile    /etc/letsencrypt/live/readur.example.com/fullchain.pem
    SSLCertificateKeyFile /etc/letsencrypt/live/readur.example.com/privkey.pem

    ProxyRequests Off
    ProxyPreserveHost On
    RequestHeader set X-Forwarded-Proto "https"

    # WebSocket (sync progress)
    RewriteEngine On
    RewriteCond %{HTTP:Upgrade} websocket [NC]
    RewriteRule ^/?(.*) "ws://127.0.0.1:8000/$1" [P,L]

    ProxyPass        / http://127.0.0.1:8000/
    ProxyPassReverse / http://127.0.0.1:8000/

    ProxyTimeout 300
    # At least MAX_FILE_SIZE_MB (here 100 MB)
    LimitRequestBody 104857600
</VirtualHost>
```

Enable the required modules and the site:

```bash
sudo a2enmod proxy proxy_http proxy_wstunnel headers rewrite ssl
sudo a2ensite readur
sudo systemctl reload apache2
```

`mod_proxy` adds `X-Forwarded-For` automatically.

## Caddy

Caddy obtains certificates automatically and sets `X-Forwarded-For`, `X-Forwarded-Proto` and `X-Forwarded-Host` and passes WebSocket upgrades by default.

```caddy
# /etc/caddy/Caddyfile
readur.example.com {
    encode gzip zstd

    request_body {
        max_size 100MB
    }

    reverse_proxy 127.0.0.1:8000 {
        health_uri /api/health
        health_interval 30s
    }
}
```

## Traefik

```yaml
# docker-compose.yml
services:
  traefik:
    image: traefik:v3.1
    command:
      - "--providers.docker=true"
      - "--providers.docker.exposedbydefault=false"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.web.http.redirections.entrypoint.to=websecure"
      - "--entrypoints.websecure.address=:443"
      - "--certificatesresolvers.letsencrypt.acme.tlschallenge=true"
      - "--certificatesresolvers.letsencrypt.acme.email=admin@example.com"
      - "--certificatesresolvers.letsencrypt.acme.storage=/letsencrypt/acme.json"
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
      - ./letsencrypt:/letsencrypt

  readur:
    image: ghcr.io/readur/readur:latest
    environment:
      PUBLIC_URL: https://readur.example.com
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.readur.rule=Host(`readur.example.com`)"
      - "traefik.http.routers.readur.entrypoints=websecure"
      - "traefik.http.routers.readur.tls.certresolver=letsencrypt"
      - "traefik.http.routers.readur.middlewares=readur-compress"
      - "traefik.http.middlewares.readur-compress.compress=true"
      - "traefik.http.services.readur.loadbalancer.server.port=8000"
```

Traefik passes WebSocket upgrades and sets `X-Forwarded-*` headers by default, and does not limit request body size unless you add a buffering middleware. The rest of the Readur service (database, volumes) is as in the [Docker Compose setup](../quickstart/docker.md).

## HAProxy

```haproxy
# /etc/haproxy/haproxy.cfg
defaults
    mode http
    option forwardfor
    timeout connect 5s
    timeout client  300s
    timeout server  300s
    timeout tunnel  1h

frontend readur_frontend
    bind *:80
    bind *:443 ssl crt /etc/haproxy/certs/readur.pem
    http-request redirect scheme https unless { ssl_fc }
    http-request set-header X-Forwarded-Proto https if { ssl_fc }
    compression algo gzip
    compression type text/html text/plain text/css application/javascript application/json
    default_backend readur

backend readur
    option httpchk GET /api/health
    server readur1 127.0.0.1:8000 check
```

`timeout tunnel` keeps the sync-progress WebSocket open.

## Certificates with Let's Encrypt

Caddy and Traefik obtain certificates themselves. For NGINX or Apache, use Certbot:

```bash
sudo apt-get install certbot python3-certbot-nginx
sudo certbot --nginx -d readur.example.com

# Check that renewal works (Certbot installs its own renewal timer)
sudo certbot renew --dry-run
```

## One Readur Server

Run one Readur server per database. It runs the OCR queue, watch folders and source syncs inside the server process and is not designed to sit behind a load balancer as several replicas, so configure a single upstream (see [Performance Tuning](./performance.md#scaling)).

## Troubleshooting

**502 Bad Gateway:** check that Readur is running and reachable from the proxy:

```bash
curl http://localhost:8000/api/health
# {"status":"ok"}

docker compose logs readur
```

From another container, use the service name (`http://readur:8000/api/health`) rather than `localhost`.

**413 Request Entity Too Large:** raise the proxy's body size limit (`client_max_body_size`, `LimitRequestBody`, `max_size`). Readur itself rejects bodies larger than `MAX_FILE_SIZE_MB`.

**Sync progress does not update live:** the WebSocket upgrade is not passed through; check the `Upgrade`/`Connection` headers (NGINX) or `proxy_wstunnel` (Apache).

**OIDC login returns to the wrong address:** set `PUBLIC_URL` to the address users reach Readur at.

**Failed logins from different users are rate limited together:** the proxy is not sending `X-Forwarded-For`, or its address is not in `TRUSTED_PROXIES`.

## Related Documentation

- [Reverse Proxy Configuration Guide](../REVERSE_PROXY.md)
- [Authentication](./authentication.md)
- [Performance Tuning](./performance.md)
- [Mozilla SSL Configuration Generator](https://ssl-config.mozilla.org/)
- [NGINX Documentation](https://nginx.org/en/docs/)
