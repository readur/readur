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
ENABLE_REGISTRATION=false  # Disable public registration
REQUIRE_EMAIL_VERIFICATION=true
PASSWORD_MIN_LENGTH=12
PASSWORD_REQUIRE_SPECIAL=true
PASSWORD_REQUIRE_NUMBERS=true
SESSION_LIFETIME_HOURS=24
```

#### User Management

Create and manage users via the API:

```bash
# Create admin user via API
curl -X POST http://localhost:8000/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "email": "admin@company.com",
    "password": "SecurePass123!",
    "role": "admin"
  }'

# Create regular user via API
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
# Set ENABLE_REGISTRATION=true in environment
```

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
```

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

Connect to Active Directory or OpenLDAP servers.

#### Basic LDAP Configuration

```bash
AUTH_METHOD=ldap
LDAP_SERVER=ldap://ldap.company.com:389
LDAP_BIND_DN=cn=readur,ou=services,dc=company,dc=com
LDAP_BIND_PASSWORD=bind-password
LDAP_BASE_DN=ou=users,dc=company,dc=com
LDAP_USER_FILTER=(uid={username})
LDAP_USER_ATTR_MAP={"email": "mail", "name": "cn"}
LDAP_GROUP_SEARCH_BASE=ou=groups,dc=company,dc=com
LDAP_GROUP_FILTER=(member={user_dn})
LDAP_ADMIN_GROUP=cn=readur-admins,ou=groups,dc=company,dc=com
```

#### Active Directory Configuration

```bash
AUTH_METHOD=ldap
LDAP_SERVER=ldaps://ad.company.com:636
LDAP_BIND_DN=readur@company.com
LDAP_BIND_PASSWORD=service-account-password
LDAP_BASE_DN=DC=company,DC=com
LDAP_USER_FILTER=(&(objectClass=user)(sAMAccountName={username}))
LDAP_USER_ATTR_MAP={"email": "mail", "name": "displayName"}
LDAP_GROUP_SEARCH_BASE=DC=company,DC=com
LDAP_GROUP_FILTER=(&(objectClass=group)(member={user_dn}))
LDAP_ADMIN_GROUP=CN=Readur Admins,OU=Groups,DC=company,DC=com
LDAP_USE_TLS=true
LDAP_TLS_VERIFY=true
```

### SAML2 Authentication

For organizations using SAML identity providers.

#### Configuration

```bash
AUTH_METHOD=saml2
SAML2_IDP_METADATA_URL=https://idp.company.com/metadata
SAML2_SP_ENTITY_ID=https://readur.company.com
SAML2_SP_ACS_URL=https://readur.company.com/saml/acs
SAML2_SP_X509_CERT=/etc/readur/saml/cert.pem
SAML2_SP_PRIVATE_KEY=/etc/readur/saml/key.pem
SAML2_ATTRIBUTE_MAPPING={"email": "EmailAddress", "name": "DisplayName"}
SAML2_ADMIN_GROUP=readur-administrators
```

## Multi-Factor Authentication

### TOTP (Time-based One-Time Password)

Enable 2FA using authenticator apps:

```bash
# Enable TOTP
MFA_ENABLED=true
MFA_METHOD=totp
MFA_ISSUER=Readur
MFA_ENFORCE_FOR_ADMINS=true
MFA_GRACE_PERIOD_DAYS=7  # Days before enforcement
```

### WebAuthn/FIDO2

Hardware security key support:

```bash
MFA_ENABLED=true
MFA_METHOD=webauthn
WEBAUTHN_RP_ID=readur.company.com
WEBAUTHN_RP_NAME=Readur Document Management
WEBAUTHN_REQUIRE_ATTESTATION=direct
```

## Role-Based Access Control (RBAC)

### Role Configuration

Define roles and permissions:

```yaml
# roles.yaml
roles:
  admin:
    permissions:
      - documents.*
      - users.*
      - settings.*
      - system.*
    
  editor:
    permissions:
      - documents.create
      - documents.read
      - documents.update
      - documents.delete.own
      - ocr.*
    
  viewer:
    permissions:
      - documents.read
      - search.*
      
  auditor:
    permissions:
      - documents.read
      - audit.view
      - reports.generate
```

### Group Mapping

Map external groups to Readur roles:

```bash
# OIDC group mapping
OIDC_GROUP_ROLE_MAPPING='{
  "readur-admins": "admin",
  "readur-editors": "editor",
  "readur-viewers": "viewer",
  "compliance-team": "auditor"
}'

# LDAP group mapping
LDAP_GROUP_ROLE_MAPPING='{
  "CN=Readur Admins,OU=Groups,DC=company,DC=com": "admin",
  "CN=Readur Users,OU=Groups,DC=company,DC=com": "editor"
}'
```

## Session Management

### Session Configuration

```bash
# Session settings
SESSION_COOKIE_NAME=readur_session
SESSION_COOKIE_SECURE=true  # HTTPS only
SESSION_COOKIE_HTTPONLY=true
SESSION_COOKIE_SAMESITE=Lax
SESSION_LIFETIME_HOURS=8
SESSION_IDLE_TIMEOUT_MINUTES=30
SESSION_REMEMBER_ME_DAYS=30
SESSION_MAX_CONCURRENT=3  # Max sessions per user
```

### Session Storage

```bash
# Redis-backed sessions (recommended)
SESSION_BACKEND=redis
REDIS_SESSION_URL=redis://localhost:6379/1

# Database sessions
SESSION_BACKEND=database
SESSION_CLEANUP_INTERVAL_HOURS=24
```

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

### API Key Authentication

For service-to-service communication:

```bash
# Enable API keys
API_KEY_ENABLED=true
API_KEY_HEADER=X-API-Key
API_KEY_EXPIRY_DAYS=90
```

Generate API keys via the API:

```bash
# Create API key for service (requires admin authentication)
curl -X POST http://localhost:8000/api/keys \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "backup-service",
    "scope": ["documents.read"]
  }'
```

### JWT Tokens

For programmatic access:

```bash
# JWT configuration
JWT_ENABLED=true
JWT_SECRET_KEY=your-jwt-secret
JWT_ALGORITHM=HS256
JWT_EXPIRY_MINUTES=60
JWT_REFRESH_ENABLED=true
JWT_REFRESH_EXPIRY_DAYS=7
```

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

# Monitor authentication metrics (if metrics endpoint enabled)
curl http://localhost:8000/metrics | grep -E "(auth|login|session)"
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