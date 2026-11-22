# Security Best Practices Guide

This guide describes the security features Readur has, the settings that control them, and what you should do around them when you run Readur in production. All settings are environment variables; the complete list is in [Security and Access Control](configuration-reference.md#security-and-access-control).

## Authentication and Authorization

### Authentication Setup

Every API request except a few public ones (health check, sign-in, public shared links) needs `Authorization: Bearer <token>`, where the token is either a session token from logging in or a personal API key. Readur does not use cookies for authentication.

#### Local Authentication

Users log in with a username and password and receive a session token (a JWT signed with HS256).

```yaml
# Optional: JWT signing secret (min 32 bytes). Generate with: openssl rand -hex 32
JWT_SECRET: "<output of: openssl rand -hex 32>"
JWT_TTL_HOURS: "12"            # token lifetime, 1-720 (default 12)
ALLOW_REGISTRATION: "false"    # self-registration (default false)
ALLOW_LOCAL_AUTH: "true"       # false = OIDC login only
```

- **Signing key.** When `JWT_SECRET` is unset, Readur generates a random signing key on first start and stores it in the database; `readur rotate-jwt-secret` replaces it (restart the servers afterwards; all sessions end). When `JWT_SECRET` is set to a value of at least 32 bytes that is not a published example value, it takes precedence. A shorter or example value is ignored with a warning in the startup log, and the database-stored key is used instead; `READUR_INSECURE_DEV_MODE=true` makes Readur use the weak value anyway, for throwaway local development only.
- **Token checks.** Every request verifies the signature, expiry, issuer and audience of the token, and that the account is still active. There are no refresh tokens; after `JWT_TTL_HOURS` the user logs in again.
- **Revocation.** Logging out (`POST /api/auth/logout`), changing a password, an administrator resetting a password, and deactivating an account all end every session of that user immediately.
- **Passwords.** Passwords must be 8 characters to 72 bytes long and are stored as bcrypt hashes (cost 12).
- **Initial admin account.** `ADMIN_USERNAME` (default `admin`), `ADMIN_EMAIL` (default `<username>@localhost`) and `ADMIN_PASSWORD`. If `ADMIN_PASSWORD` is unset, a random password is written with mode `0600` to `initial-admin-password` in the `.readur` directory inside `UPLOAD_PATH` (override with `ADMIN_PASSWORD_FILE`); it is not written to the logs. Change the password after the first login and delete the file. `readur reset-admin-password` (in the Docker image: `/app/readur reset-admin-password`) sets a new one if it is lost (see [CLI Tools](administration/cli-tools.md)).
- **Registration.** When `ALLOW_REGISTRATION=true`, self-registered accounts are created inactive and need administrator approval before they can sign in.

The web interface keeps its session token in the browser's local storage.

#### OIDC/SSO Integration

Readur can sign users in through an OpenID Connect provider (see the [OIDC Setup Guide](oidc-setup.md)):

```yaml
OIDC_ENABLED: "true"
OIDC_CLIENT_ID: "readur-client"
OIDC_CLIENT_SECRET: "your-client-secret"
OIDC_ISSUER_URL: "https://auth.example.com/realms/readur"
OIDC_REDIRECT_URI: "https://readur.example.com/api/auth/oidc/callback"
PUBLIC_URL: "https://readur.example.com"
OIDC_LINK_EXISTING_BY_EMAIL: "false"
OIDC_AUTO_REGISTER: "false"
```

- Every login uses PKCE (S256), a random `state` and a random `nonce`; the ID token's signature, issuer, audience, expiry and nonce are verified. Readur requests the scopes `openid email profile`.
- After the provider returns, Readur redirects the browser to `<PUBLIC_URL>/auth/callback` with a short-lived, single-use code in the URL fragment; the session token itself never appears in a URL. `PUBLIC_URL` is used to build this redirect (request headers are not used).
- An OIDC login is linked to an existing local account with the same email only when `OIDC_LINK_EXISTING_BY_EMAIL=true` and the provider reports `email_verified: true`. Unknown users get an account only when `OIDC_AUTO_REGISTER=true`.
- Set `ALLOW_LOCAL_AUTH=false` to turn off password login entirely.

### Role-Based Access Control

Readur has two roles:

| Role | Permissions |
|------|------------|
| Admin | Everything a user can do, plus: all users' documents, user management, all users' shared links and API keys (list and revoke), deleting any comment, `/api/metrics` and `/metrics` |
| User | Their own documents, sources, labels, settings, shared links and API keys; comments on documents they can access |

Each request reads the user's current role from the database, so promoting or demoting a user takes effect on their next request. See the [User Management Guide](user-management-guide.md).

Local folder sources read directories on the server. Unless `LOCAL_SOURCE_ALLOWED_PATHS` is set, only administrators can create them; when it is set, every local folder source (including an administrator's) must lie inside one of the listed directories.

## Shared Link Security

Shared links let unauthenticated people view or download one document through a token URL.

### Token Generation

Shared link tokens are 256-bit (32-byte) values from a cryptographically secure random number generator, encoded as URL-safe base64. They cannot practically be guessed or enumerated.

### Access Controls

- **Password protection**: An optional password, stored as a bcrypt hash, is required before viewing or downloading. It is sent in a POST body, never in the URL.
- **Expiration**: A link can have an expiry time, after which it answers `410 Gone`.
- **View limits**: A link can be limited to a number of views or downloads; after that it answers `410 Gone`.
- **Revocation**: The link's creator (or an administrator) can revoke it at once; a revoked link answers `410 Gone`.

### Authorization Model

- **Creating links**: Requires login. Users can create links only for documents they can access (their own; administrators any document).
- **Opening links**: No login. The token is the credential, with the optional password as a second factor.
- **Managing links**: Users see and revoke their own links. Administrators see and can revoke all links.

### Brute-Force Protection

Password checks are limited to 10 per minute and other shared link requests to 60 per minute from one address; more get `429 Too Many Requests`.

### Content-Disposition Sanitization

Filenames in download and view responses are reduced to letters, digits, dots, hyphens, underscores and spaces (other characters become `_`) to prevent header injection.

## API Key Security

Personal API keys (see [API Key Authentication](api-reference.md#api-key-authentication)) grant programmatic access with the permissions of the user who created them. Because they live longer than session tokens, they need corresponding care.

### How Readur protects keys

- **Entropy**: 256 bits (32 bytes) of OS randomness per key, encoded as URL-safe base64 after a fixed `readur_pat_` prefix.
- **At-rest storage**: Only the SHA-256 hash of the key is stored. The plaintext is returned exactly once, in the `POST /api/auth/keys` response. The stored `key_prefix` holds only the first 12 characters, so the interface can identify a key without revealing it.
- **Hashing choice**: SHA-256 rather than bcrypt is deliberate. Slow password hashes compensate for low-entropy, human-chosen passwords; guessing a 256-bit random token is already infeasible.
- **Uniform failures**: An unknown, revoked or expired key, or a key of a deactivated user, all get the same `401 Unauthorized`.
- **Revocation**: Revoke a key in **Settings → API keys** or with `DELETE /api/auth/keys/{id}`; it stops working on the next request. Revoked keys are kept (marked with `revoked_at`) so their `last_used_at` history survives.
- **Role changes**: Each request re-reads the owner's role and status, so demoting or deactivating the user takes effect immediately on all of their keys.
- **Expiration**: Optional, at most 365 days. The interface suggests 90 days.
- **Limits**: 10 new keys per hour per user, and at most 20 active keys per user.
- **Secret scanning**: The `readur_pat_` prefix makes accidentally committed keys easy for secret scanners to recognise.

### What you must do

- **Treat keys like passwords.** A leaked key is as good as the owner's login.
- **Use TLS only.** Readur itself serves plain HTTP and accepts a key over it; put it behind an HTTPS reverse proxy (see [TLS Configuration](#tls-configuration)).
- **Store keys in a secret manager** or your CI system's encrypted variables, not in committed files.
- **Give each integration its own key.** Then `last_used_at` tells you which integration is active, and revoking one does not break the others.
- **Prefer short expirations** and rotate: create a new key, deploy it, then revoke the old one.
- **Don't run automated jobs as administrators.** Create a separate non-admin user for integrations that do not need admin rights.

### For administrators

- `GET /api/auth/keys?all=true` lists every user's keys, and `DELETE /api/auth/keys/{id}` revokes any key. During an incident, find the keys of the compromised account, revoke them, and have the user create new ones.
- Watch `last_used_at` (updated at most once a minute per key) for keys that are no longer used, and revoke them.
- Successful API key logins are logged at debug level with only `api_key_id`, `user_id` and `key_prefix`, never the key or its hash.

## File Upload Security

- **Size limit**: `MAX_FILE_SIZE_MB` (default 50) caps every request body, uploads included; larger requests get `413 Payload Too Large`.
- **Storage names**: Uploaded files are stored under the document's generated ID (keeping the extension), not under the uploaded filename, and paths are checked to stay inside the storage directory.
- **Serving files back**: Downloads and previews are sent with `X-Content-Type-Options: nosniff`. Types a browser could run as active content (HTML, SVG, XML, JavaScript, and unknown types) are always sent as attachments, and every type except PDF also gets a sandboxing `Content-Security-Policy`, so an uploaded file cannot run scripts on Readur's origin.
- **File types**: Uploads through the web interface and API are not limited by type. `ALLOWED_FILE_TYPES` limits only what is imported from watch folders and by batch ingestion. Readur does not scan files for malware; scan them before they reach Readur if you need that.

## Network Security

### TLS Configuration

Readur serves plain HTTP on port 8000 and does not terminate TLS itself. Always put it behind a reverse proxy that serves HTTPS (see the [Reverse Proxy guide](REVERSE_PROXY.md)), and set `PUBLIC_URL` to the `https://` address. When `PUBLIC_URL` starts with `https://`, Readur adds `Strict-Transport-Security: max-age=31536000` to its responses.

```nginx
server {
    listen 443 ssl;
    http2 on;
    server_name readur.example.com;
    ssl_certificate /etc/ssl/certs/readur.crt;
    ssl_certificate_key /etc/ssl/private/readur.key;
    ssl_protocols TLSv1.2 TLSv1.3;

    client_max_body_size 50M;   # at least MAX_FILE_SIZE_MB

    location / {
        proxy_pass http://127.0.0.1:8000;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        # WebSocket for sync progress
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

Do not expose port 8000 or the PostgreSQL port to the internet. The PostgreSQL connection can be encrypted by adding `sslmode=require` (or `verify-full`) to `DATABASE_URL`, for example `postgresql://readur:password@db.example.com:5432/readur?sslmode=require`.

### CORS Configuration

By default no cross-origin requests are allowed; the bundled frontend is served from the same origin and needs no CORS configuration. To allow other origins, list them explicitly:

```yaml
CORS_ALLOWED_ORIGINS: "https://app.example.com,https://admin.example.com"
```

The server refuses to start if an entry is not a valid `http(s)://host[:port]` origin.

### Rate Limiting

Readur limits these requests in memory, per server instance (the limits are fixed; there are no settings for them):

- Failed logins: 10 per 15 minutes for one username from one address, 100 per 15 minutes for one username, 50 per 15 minutes from one address. Only wrong passwords count.
- Self-registrations: 10 per hour from one address.
- OIDC login starts and code exchanges: 30 per minute from one address.
- Shared links: 10 password checks and 60 other requests per minute from one address; 20 new links per hour per user.
- Comments: 10 per minute per user. API keys: 10 new keys per hour per user.

A limited request gets `429 Too Many Requests`. The authentication endpoints send a `Retry-After` header; the shared link, comment and API key endpoints state the wait in the error message. See [Rate Limiting](api-reference.md#rate-limiting) for the endpoints each limit covers.

Per-address limits use the client address. `X-Forwarded-For` is honoured only for requests arriving from a trusted proxy: by default any loopback or private-network address (`127.0.0.0/8`, `::1/128`, `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `fc00::/7`). Set `TRUSTED_PROXIES` to your proxy's IPs or CIDRs to narrow this, or to `none` if clients reach Readur directly from a private network.

### Metrics Endpoint

`/metrics` (Prometheus format) and `/api/metrics` (JSON) need an administrator's session or API key. For a Prometheus scraper, set `METRICS_TOKEN` (at least 16 characters) and send it as `Authorization: Bearer <METRICS_TOKEN>`; it grants access to `/metrics` only.

## Security Headers

Readur sets these headers on its responses (a handler that sets its own value keeps it), so you do not need to add them in the reverse proxy:

| Header | Value |
|--------|-------|
| `Content-Security-Policy` | `default-src 'self'`, with exceptions for inline styles, Google Fonts, `data:` and `blob:` images, `blob:` frames and workers, and WebSockets; `frame-ancestors 'self'` |
| `X-Content-Type-Options` | `nosniff` |
| `X-Frame-Options` | `SAMEORIGIN` |
| `Referrer-Policy` | `no-referrer` |
| `Cross-Origin-Opener-Policy` | `same-origin` |
| `Strict-Transport-Security` | `max-age=31536000`, only when `PUBLIC_URL` starts with `https://` |

If your proxy adds its own copies of these headers, make sure they do not conflict with Readur's.

## Common Vulnerabilities and Mitigations

- **CSRF**: Authentication uses a bearer token in the `Authorization` header, which browsers do not attach to cross-site requests on their own, so no CSRF tokens are needed.
- **SQL injection**: Database queries use bound parameters.
- **Directory traversal**: Stored file names are generated, and file paths are checked to stay inside the storage directory. Local folder sources are restricted as described under [Role-Based Access Control](#role-based-access-control).
- **Stored secrets in API responses**: Source responses never include WebDAV passwords or S3 secret keys (they report `has_password` / `has_secret_access_key` instead), and the settings response reports `has_webdav_password`.

## Secrets Management

### Environment Variables

Readur's secrets are `JWT_SECRET` (optional), `ADMIN_PASSWORD`, `DATABASE_URL` or `POSTGRES_PASSWORD`, `OIDC_CLIENT_SECRET`, `S3_SECRET_ACCESS_KEY` and `METRICS_TOKEN`. Keep them out of version control: put them in a `.env` file that is not committed, in Docker or Kubernetes secrets, or in a secret manager that injects them as environment variables. Variables set in the environment take precedence over `.env`.

With the Helm chart, the initial admin password (and an explicit `JWT_SECRET`, if you set one) live in a Secret; see [Helm Chart Values](configuration-reference.md#helm-chart-values).

### Secret Rotation

- **JWT signing key**: with a database-stored key, run `readur rotate-jwt-secret` (`/app/readur rotate-jwt-secret` in the Docker image) and restart every server. With `JWT_SECRET`, set a new value and restart. Either way, all users are signed out.
- **Admin password**: change it in the web interface, or run `readur reset-admin-password`.
- **API keys**: create a new key, deploy it, revoke the old one.
- **S3, database and OIDC credentials**: change them at the provider, update the environment variables and restart Readur.

## Monitoring and Logging

Readur logs to standard output; `RUST_LOG` sets the level (see [Configuration Reference](configuration-reference.md#server-and-database)). It has no audit log, and failed password logins are not logged (they only count towards the rate limits). OIDC login failures and rejected OIDC users are logged as warnings. Collect the container logs if you want to keep them. For metrics and health checks, see the [Health Monitoring Guide](health-monitoring-guide.md).

## Security Checklist

- [ ] Readur is reachable only through an HTTPS reverse proxy, and `PUBLIC_URL` is set to the `https://` address
- [ ] The initial admin password was changed and the generated password file deleted
- [ ] `ALLOW_REGISTRATION` is `false` unless you need self-registration
- [ ] `JWT_SECRET`, if set, is a random value of at least 32 bytes
- [ ] `TRUSTED_PROXIES` names your proxy (or `none`)
- [ ] `CORS_ALLOWED_ORIGINS` is empty unless another origin must call the API
- [ ] `LOCAL_SOURCE_ALLOWED_PATHS` is set if users other than admins should create local folder sources
- [ ] PostgreSQL is not exposed publicly and uses a strong password
- [ ] Integrations use their own non-admin API keys with expiry dates
- [ ] Backups of the database and the upload directory are tested (see [Backup](self-hosting/backup.md))
- [ ] The Readur image is updated regularly
