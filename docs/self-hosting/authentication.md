# Authentication Configuration

## Overview

Readur supports multiple authentication methods to integrate with your existing identity infrastructure. This guide covers configuration for local authentication, OIDC/SSO, LDAP, and multi-factor authentication.

## Authentication Methods

### Local Authentication

Default authentication using Readur's built-in user management.

#### Basic Configuration

```bash
# In .env file
AUTH_METHOD=local
ALLOW_REGISTRATION=false  # Self-registration (default: false)
REQUIRE_EMAIL_VERIFICATION=true
PASSWORD_MIN_LENGTH=12
PASSWORD_REQUIRE_SPECIAL=true
PASSWORD_REQUIRE_NUMBERS=true
JWT_TTL_HOURS=12  # Login token lifetime (1-720, default 12)
```

#### User Management

Create and manage users via the API:

The initial admin account is created on first startup from `ADMIN_USERNAME` (default `admin`), `ADMIN_EMAIL` (default `<username>@localhost`) and `ADMIN_PASSWORD`. If `ADMIN_PASSWORD` is unset, a random password is written to `initial-admin-password` in the `.readur` directory inside the upload directory (see `ADMIN_PASSWORD_FILE`) rather than to the logs.

```bash
# Create a user via the admin API
curl -X POST http://localhost:8000/api/users \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "john",
    "email": "john@company.com",
    "password": "UserPass456!",
    "role": "user"
  }'

# Users can also self-register if enabled:
# Set ALLOW_REGISTRATION=true in environment
```

When `ALLOW_REGISTRATION=true`, `POST /api/auth/register` creates accounts with the **User** role in a disabled state. An administrator approves an account by activating it in **Settings > User Management** (or `PUT /api/users/{id}` with `{"is_active": true}`); until then the account cannot sign in.

Signed-in users can change their own password with `POST /api/auth/password`.

### OIDC/OAuth2 (Recommended)

Integrate with enterprise identity providers for single sign-on.

#### Generic OIDC Configuration

```bash
# In .env file
AUTH_METHOD=oidc
OIDC_ISSUER=https://auth.company.com
OIDC_CLIENT_ID=readur-app
OIDC_CLIENT_SECRET=your-client-secret
OIDC_REDIRECT_URI=https://readur.company.com/auth/callback
OIDC_SCOPE=openid profile email
OIDC_USER_CLAIM=email
OIDC_GROUPS_CLAIM=groups
OIDC_ADMIN_GROUP=readur-admins
PUBLIC_URL=https://readur.company.com       # base URL for the post-login redirect
OIDC_LINK_EXISTING_BY_EMAIL=false           # see below
```

`PUBLIC_URL` sets the base URL Readur redirects to after login; if unset, the origin of `OIDC_REDIRECT_URI` is used. Request headers are not used to derive it. By default an OIDC login is not linked to an existing local account with the same email. Set `OIDC_LINK_EXISTING_BY_EMAIL=true` to link them; linking only happens when the identity provider reports `email_verified: true`. See [OIDC Setup](../oidc-setup.md) for details.

#### Keycloak Integration

```bash
AUTH_METHOD=oidc
OIDC_ISSUER=https://keycloak.company.com/realms/master
OIDC_CLIENT_ID=readur
OIDC_CLIENT_SECRET=abc123def456
OIDC_REDIRECT_URI=https://readur.company.com/auth/callback
OIDC_SCOPE=openid profile email roles
OIDC_USER_CLAIM=preferred_username
OIDC_GROUPS_CLAIM=realm_access.roles
OIDC_ADMIN_GROUP=readur-admin
OIDC_AUTO_CREATE_USERS=true
```

#### Auth0 Integration

```bash
AUTH_METHOD=oidc
OIDC_ISSUER=https://company.auth0.com/
OIDC_CLIENT_ID=your-client-id
OIDC_CLIENT_SECRET=your-client-secret
OIDC_REDIRECT_URI=https://readur.company.com/auth/callback
OIDC_SCOPE=openid profile email
OIDC_AUDIENCE=https://readur.company.com/api
OIDC_USER_CLAIM=email
OIDC_GROUPS_CLAIM=https://readur.com/groups
```

#### Azure AD Integration

```bash
AUTH_METHOD=oidc
OIDC_ISSUER=https://login.microsoftonline.com/{tenant-id}/v2.0
OIDC_CLIENT_ID=application-id
OIDC_CLIENT_SECRET=client-secret
OIDC_REDIRECT_URI=https://readur.company.com/auth/callback
OIDC_SCOPE=openid profile email User.Read
OIDC_USER_CLAIM=upn
OIDC_GROUPS_CLAIM=groups
OIDC_ADMIN_GROUP=readur-administrators
OIDC_TOKEN_ENDPOINT_AUTH_METHOD=client_secret_post
```

#### Google Workspace Integration

```bash
AUTH_METHOD=oidc
OIDC_ISSUER=https://accounts.google.com
OIDC_CLIENT_ID=your-client-id.apps.googleusercontent.com
OIDC_CLIENT_SECRET=your-client-secret
OIDC_REDIRECT_URI=https://readur.company.com/auth/callback
OIDC_SCOPE=openid profile email
OIDC_USER_CLAIM=email
OIDC_HOSTED_DOMAIN=company.com  # Restrict to company domain
```

### LDAP Authentication

> **Note:** LDAP authentication is not currently implemented. This section is reserved for a planned future feature. Use OIDC/OAuth2 to integrate with LDAP-backed identity providers (most IdPs like Keycloak and Azure AD can front LDAP directories).

### SAML2 Authentication

> **Note:** SAML2 authentication is not currently implemented. This section is reserved for a planned future feature. Use OIDC/OAuth2 instead, which is supported by all major identity providers.

## Multi-Factor Authentication

> **Note:** Built-in MFA (TOTP, WebAuthn) is not currently implemented. To enforce MFA, configure it at your OIDC identity provider level — most providers (Keycloak, Auth0, Azure AD, Google Workspace) support MFA policies that will apply when users authenticate to Readur via SSO.

## Role-Based Access Control (RBAC)

### Roles

Readur has two built-in roles:

| Role | Description |
|------|------------|
| **Admin** | Full access to all documents, users, settings, and system configuration. Can manage all shared links and moderate all comments. |
| **User** | Access to own documents only. Can create shared links for own documents, comment on accessible documents, and manage own profile. |

New users created via local registration default to the **User** role. Admins can promote users via the user management API.

When using OIDC, role assignment happens at user creation/login time based on your IdP configuration. You can configure an admin group claim to automatically assign the Admin role to members of a specific group in your identity provider.

## Session Management

Readur uses JWT tokens for authentication. Tokens are valid for `JWT_TTL_HOURS` hours from issuance (default 12, allowed range 1-720). There are no refresh tokens — users must re-authenticate after token expiry. Logging out (`POST /api/auth/logout`) revokes all of the user's sessions, not only the current one.

```bash
# JWT configuration
JWT_SECRET=<output of: openssl rand -hex 32>  # Optional, min 32 bytes when set
JWT_TTL_HOURS=12                              # Optional
```

The server refuses to start if `JWT_SECRET` is unset, shorter than 32 bytes, or a published example value. Generate a secret with:
```bash
openssl rand -hex 32
```

For throwaway local development only, `READUR_INSECURE_DEV_MODE=true` allows startup with a weak secret. Do not set it in production.

Tokens are stored in the browser's `localStorage` and sent as `Authorization: Bearer <token>` headers on API requests.

## Security Headers

Configure security headers for authentication:

```bash
# Security headers
SECURITY_HEADERS_ENABLED=true
SECURITY_CSP="default-src 'self'; script-src 'self' 'unsafe-inline'"
SECURITY_HSTS_SECONDS=31536000
SECURITY_HSTS_INCLUDE_SUBDOMAINS=true
SECURITY_X_FRAME_OPTIONS=DENY
SECURITY_X_CONTENT_TYPE_OPTIONS=nosniff
SECURITY_REFERRER_POLICY=strict-origin-when-cross-origin
```

## API Authentication

For programmatic access (scripts, integrations), authenticate using the same JWT flow as the web UI:

```bash
# Obtain a token
TOKEN=$(curl -s -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "your-password"}' | jq -r '.token')

# Use the token in subsequent requests
curl http://localhost:8000/api/documents \
  -H "Authorization: Bearer $TOKEN"
```

Tokens expire after 24 hours. There is no refresh token mechanism — obtain a new token when needed.

> **Note:** Dedicated API key authentication (service-to-service with scoped keys) is not currently implemented. Use JWT tokens for all API access.

## Troubleshooting

### OIDC Issues

#### Discovery Failed

```bash
# Test OIDC discovery
curl https://auth.company.com/.well-known/openid-configuration

# Check network connectivity
docker-compose exec readur curl https://auth.company.com
```

#### Token Validation Errors

```bash
# Enable debug logging
OIDC_DEBUG=true
LOG_LEVEL=DEBUG

# Check token claims
docker-compose exec readur python manage.py debug_oidc_token
```

### LDAP Issues

#### Connection Failed

```bash
# Test LDAP connection
ldapsearch -x -H ldap://ldap.company.com:389 \
  -D "cn=readur,ou=services,dc=company,dc=com" \
  -W -b "dc=company,dc=com" "(uid=testuser)"

# Test from container
docker-compose exec readur ldapsearch -x -H $LDAP_SERVER
```

#### User Not Found

```bash
# Debug LDAP queries
LDAP_DEBUG=true
LOG_LEVEL=DEBUG

# Test LDAP user filter directly
ldapsearch -x -H $LDAP_SERVER \
  -D "$LDAP_BIND_DN" \
  -w "$LDAP_BIND_PASSWORD" \
  -b "$LDAP_BASE_DN" \
  "(uid=testuser)"
```

### Session Issues

#### Sessions Expiring Too Quickly

```bash
# Check Redis connectivity
docker-compose exec readur redis-cli -h redis ping

# Monitor session creation
docker-compose exec readur redis-cli monitor | grep session
```

## Best Practices

### Security Recommendations

1. **Always use HTTPS** in production
2. **Enable MFA** for administrative accounts
3. **Regular password rotation** for service accounts
4. **Audit authentication logs** regularly
5. **Use external IdP** when possible
6. **Implement rate limiting** on authentication endpoints
7. **Monitor failed login attempts**

### Integration Testing

Test authentication before production:

```bash
# Test OIDC flow via health endpoint
curl http://localhost:8000/api/health/auth/oidc

# Test LDAP connection
curl -X POST http://localhost:8000/api/auth/test \
  -H "Content-Type: application/json" \
  -d '{
    "method": "ldap",
    "username": "testuser",
    "password": "testpass"
  }'

# Verify user groups via API
curl http://localhost:8000/api/users/testuser@company.com/groups \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

### Monitoring

Set up authentication monitoring:

```bash
# Monitor failed login attempts via logs
docker-compose logs readur | grep -E "(auth|login|failed)" | tail -n 100

# Check active sessions in database
docker-compose exec readur psql -U readur -d readur -c \
  "SELECT username, last_login, session_count FROM users WHERE last_login > NOW() - INTERVAL '24 hours';"

# Monitor authentication metrics (requires METRICS_TOKEN or an admin token)
curl -H "Authorization: Bearer $METRICS_TOKEN" http://localhost:8000/metrics | grep -E "(auth|login|session)"
```

## Migration from Other Systems

### Migrating Users

```bash
# Export users from old system
old_system_export_users > users.json

# Import to Readur via bulk API
curl -X POST http://localhost:8000/api/users/import \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d @users.json

# Or create users one by one via API
while IFS=, read -r username email role; do
  curl -X POST http://localhost:8000/api/users \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
      \"username\": \"$username\",
      \"email\": \"$email\",
      \"role\": \"$role\"
    }"
done < users.csv
```

## Related Documentation

- [User Management Guide](../user-management-guide.md)
- [Security Best Practices](../security-guide.md)
- [OIDC Setup](../oidc-setup.md)
- [API Reference](../api-reference.md)