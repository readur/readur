# Authentication Configuration

Readur signs users in with a local username and password, with single sign-on through an OpenID Connect (OIDC) provider, or both. Scripts and integrations use personal API keys. There is no LDAP, SAML or built-in multi-factor authentication; if you need those, put them in your OIDC identity provider (most providers can front an LDAP directory and enforce MFA) and sign in to Readur through OIDC.

All authentication settings are environment variables, read at startup; see the [Configuration Reference](../configuration-reference.md#security-and-access-control) for the complete list.

## Local Accounts

Local username/password login is on by default (`ALLOW_LOCAL_AUTH=true`). Passwords are stored as bcrypt hashes and must be at least 8 characters and at most 72 bytes long; there are no password rules to configure.

### The Initial Administrator

On first startup, when no account named `ADMIN_USERNAME` exists, Readur creates it with the `admin` role:

| Variable | Default | Purpose |
|----------|---------|---------|
| `ADMIN_USERNAME` | `admin` | Username of the initial administrator |
| `ADMIN_EMAIL` | `<username>@localhost` | Email of the initial administrator |
| `ADMIN_PASSWORD` | - | Password (at least 8 characters) |
| `ADMIN_PASSWORD_FILE` | `<UPLOAD_PATH>/.readur/initial-admin-password` | Where a generated password is written |

If `ADMIN_PASSWORD` is unset, a random 24-character password is generated and written to `ADMIN_PASSWORD_FILE` (mode `0600`); it is not written to the logs. In the Docker image the file is `/app/uploads/.readur/initial-admin-password`:

```bash
docker compose exec readur cat /app/uploads/.readur/initial-admin-password
```

To regain access to the administrator account later, run `readur reset-admin-password` (see [CLI Tools](../administration/cli-tools.md)).

### Creating Accounts

Administrators create accounts in **Settings → Users**, or through the API:

```bash
curl -X POST http://localhost:8000/api/users \
  -H "Authorization: Bearer $token" \
  -H "Content-Type: application/json" \
  -d '{"username": "john", "email": "john@example.com", "password": "a-long-password", "role": "user"}'
```

`role` is `user` (the default) or `admin`.

### Self-Registration

Self-registration is off by default. With `ALLOW_REGISTRATION=true` (and local login allowed), the login page offers registration and `POST /api/auth/register` creates accounts with the `user` role in an inactive state. An administrator approves an account by activating it in **Settings → Users** (or with `PUT /api/users/{id}` and `{"is_active": true}`); until then it cannot sign in.

### Changing Passwords

Signed-in users change their own password in **Settings → Account** (`POST /api/auth/password`). This ends all of their other sessions.

## Single Sign-On with OIDC

Readur works with any OIDC provider that publishes a discovery document (Keycloak, Authentik, Auth0, Azure AD / Entra ID, Google and others). It uses the authorization code flow with PKCE and requests the scopes `openid email profile`.

```bash
OIDC_ENABLED=true
OIDC_CLIENT_ID=readur
OIDC_CLIENT_SECRET=your-client-secret   # omit for a public (PKCE-only) client
OIDC_ISSUER_URL=https://auth.example.com/realms/main
OIDC_REDIRECT_URI=https://readur.example.com/api/auth/oidc/callback
PUBLIC_URL=https://readur.example.com
OIDC_AUTO_REGISTER=true
```

| Variable | Default | Purpose |
|----------|---------|---------|
| `OIDC_ENABLED` | `false` | Turn OIDC login on |
| `OIDC_CLIENT_ID` | - | Client ID registered with the provider |
| `OIDC_CLIENT_SECRET` | - | Client secret; without it Readur acts as a public client |
| `OIDC_ISSUER_URL` | - | The provider's issuer URL; endpoints are discovered from it |
| `OIDC_REDIRECT_URI` | - | Callback URL ending in `/api/auth/oidc/callback`, registered with the provider exactly as written |
| `OIDC_AUTO_REGISTER` | `false` | Create an account on the first login of an unknown user |
| `OIDC_LINK_EXISTING_BY_EMAIL` | `false` | Link an OIDC login to an existing account with the same email, only when the provider reports `email_verified: true` |
| `PUBLIC_URL` | - | Base URL the browser returns to after login; defaults to the origin of `OIDC_REDIRECT_URI` |
| `ALLOW_LOCAL_AUTH` | `true` | `false` turns off username/password login, leaving OIDC only |

Accounts created through OIDC get the `user` role and have no password; Readur does not read group or role claims, so an administrator promotes accounts in **Settings → Users**. With `OIDC_AUTO_REGISTER=false`, an unknown user is refused with the error `no_account` on the login page until an administrator creates or links the account.

If the provider cannot be reached or its discovery document does not match `OIDC_ISSUER_URL`, the server still starts with OIDC disabled and logs the reason. The server refuses to start only when OIDC is disabled and `ALLOW_LOCAL_AUTH=false`, because then nobody could sign in.

The [OIDC Setup Guide](../oidc-setup.md) has provider-specific steps and troubleshooting.

## Sessions

A successful login returns a session token, a JWT valid for `JWT_TTL_HOURS` hours (default 12, allowed 1-720). There are no refresh tokens; when a session expires, the user signs in again. The web interface keeps the token in the browser's local storage and sends it as `Authorization: Bearer <token>`. Logging out (`POST /api/auth/logout`) and changing the password end all of the user's sessions.

Tokens are signed with `JWT_SECRET` when it is set to a value of at least 32 bytes. When it is unset, Readur generates a signing key on first start and stores it in the database, shared by every server using that database. A `JWT_SECRET` that is too short or a published example value is ignored with a startup warning, and the stored key is used instead (`READUR_INSECURE_DEV_MODE=true` makes the server use it anyway; for throwaway development only).

```bash
# Generate your own secret
openssl rand -hex 32

# Or replace the key stored in the database (signs everyone out; restart the server afterwards)
readur rotate-jwt-secret
```

## API Keys

For scripts, integrations and AI assistants, each user can create personal API keys in **Settings → API keys** (or with `POST /api/auth/keys`). A key looks like `readur_pat_...`, is shown once when created, and is sent exactly like a session token:

```bash
curl http://localhost:8000/api/documents \
  -H "Authorization: Bearer readur_pat_..."
```

A key acts with all the permissions of its owner. It may expire after 1 to 365 days or never, can be revoked at any time, and is not affected by logging out. Each user may hold up to 20 active keys. See [API Key Endpoints](../api-reference.md#api-key-endpoints) for the full API.

For a quick script without an API key, exchange a username and password for a session token:

```bash
token=$(curl -s -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "your-password"}' | jq -r '.token')

curl http://localhost:8000/api/documents -H "Authorization: Bearer $token"
```

## Roles

| Role | Access |
|------|--------|
| `admin` | Everything, including user management, server configuration, the OCR queue and metrics |
| `user` | Their own documents, labels, sources, shared links, settings and API keys |

Self-registered and OIDC-created accounts get the `user` role.

## Brute-Force Protection

Failed logins are rate limited per username and client address, and a limited request gets `429 Too Many Requests` with a `Retry-After` header. The limits are fixed; see [Rate Limiting](../api-reference.md#rate-limiting). Behind a reverse proxy, set `TRUSTED_PROXIES` so the real client address is taken from `X-Forwarded-For` (see [Reverse Proxy Configuration](reverse-proxy.md)).

## Security Headers

Readur adds these headers to every response that does not set its own; they are not configurable:

| Header | Value |
|--------|-------|
| `X-Content-Type-Options` | `nosniff` |
| `X-Frame-Options` | `SAMEORIGIN` |
| `Referrer-Policy` | `no-referrer` |
| `Content-Security-Policy` | A policy for the bundled web interface (scripts from the same origin only, no framing by other sites) |
| `Cross-Origin-Opener-Policy` | `same-origin` |
| `Strict-Transport-Security` | `max-age=31536000`, only when `PUBLIC_URL` starts with `https://` |

Uploaded files are served with a restrictive sandboxing `Content-Security-Policy` (except PDFs), and active content such as HTML and SVG is always sent as a download.

Cross-origin requests are refused unless their origin is listed in `CORS_ALLOWED_ORIGINS`; the bundled web interface is same-origin and needs no entry.

## Troubleshooting

**Checking which sign-in methods are active:**

```bash
curl http://localhost:8000/api/auth/config
```

`oidc_enabled` is `true` only when the OIDC client initialized at startup; `allow_local_auth` and `allow_registration` reflect the corresponding settings.

**OIDC does not appear on the login page:** look for the OIDC lines in the startup log (`docker compose logs readur`), and check that the server can fetch the discovery document:

```bash
docker compose exec readur curl -s https://auth.example.com/realms/main/.well-known/openid-configuration
```

**Users are signed out after a restart:** a `JWT_SECRET` that changes between restarts invalidates all sessions. Leave it unset (the key stored in the database is kept) or set a fixed value.

**More detail in the logs:** set `RUST_LOG=debug` and restart.

## Related Documentation

- [OIDC Setup Guide](../oidc-setup.md)
- [User Management Guide](../user-management-guide.md)
- [Security Best Practices](../security-guide.md)
- [API Reference](../api-reference.md)
