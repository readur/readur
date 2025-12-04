# OIDC Authentication Setup Guide

This guide explains how to configure OpenID Connect (OIDC) authentication for Readur, allowing users to sign in using external identity providers like Google, Microsoft Azure AD, Keycloak, Auth0, or any OIDC-compliant provider.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Configuration](#configuration)
  - [Environment Variables](#environment-variables)
  - [Example Configurations](#example-configurations)
- [Identity Provider Setup](#identity-provider-setup)
  - [Google OAuth 2.0](#google-oauth-20)
  - [Microsoft Azure AD](#microsoft-azure-ad)
  - [Keycloak](#keycloak)
  - [Auth0](#auth0)
  - [Authentik](#authentik)
  - [Generic OIDC Provider](#generic-oidc-provider)
- [Testing the Setup](#testing-the-setup)
- [User Experience](#user-experience)
- [Troubleshooting](#troubleshooting)
- [Security Considerations](#security-considerations)

## Overview

OIDC authentication in Readur provides:

- **Single Sign-On (SSO)**: Users can sign in with existing corporate accounts
- **Email-Based User Syncing**: Automatically link existing local users to OIDC by email
- **Flexible Auto-Registration**: Control whether new users can self-register via OIDC
- **Centralized User Management**: User provisioning handled by your identity provider
- **Enhanced Security**: No need to manage passwords in Readur
- **Seamless Integration**: Works alongside existing local authentication

When OIDC is enabled, users will see a "Sign in with OIDC" button on the login page alongside the standard username/password form.

## Prerequisites

Before configuring OIDC, ensure you have:

1. **Access to an OIDC Provider**: Google, Microsoft, Keycloak, Auth0, etc.
2. **Ability to Register Applications**: Admin access to create OAuth2/OIDC applications
3. **Network Connectivity**: Readur server can reach the OIDC provider endpoints
4. **SSL/TLS Setup**: HTTPS is strongly recommended for production deployments

## Configuration

### Environment Variables

Configure OIDC by setting these environment variables:

| Variable | Required | Description | Example |
|----------|----------|-------------|---------|
| `OIDC_ENABLED` | ✅ | Enable OIDC authentication | `true` |
| `OIDC_CLIENT_ID` | ✅ | OAuth2 client ID from your provider | `readur-app-client-id` |
| `OIDC_CLIENT_SECRET` | ✅ | OAuth2 client secret from your provider | `very-secret-key` |
| `OIDC_ISSUER_URL` | ✅ | OIDC provider's issuer URL | `https://accounts.google.com` |
| `OIDC_REDIRECT_URI` | ✅ | Callback URL for your Readur instance | `https://readur.company.com/api/auth/oidc/callback` |
| `OIDC_AUTO_REGISTER` | ❌ | Allow new users to self-register (default: `false`) | `true` or `false` |
| `ALLOW_LOCAL_AUTH` | ❌ | Allow username/password authentication (default: `true`) | `true` or `false` |

### Example Configurations

#### Basic OIDC Setup

```env
# Enable OIDC
OIDC_ENABLED=true

# Provider settings (example for Google)
OIDC_CLIENT_ID=123456789-abcdefgh.apps.googleusercontent.com
OIDC_CLIENT_SECRET=GOCSPX-your-secret-key
OIDC_ISSUER_URL=https://accounts.google.com
OIDC_REDIRECT_URI=https://readur.company.com/api/auth/oidc/callback
OIDC_AUTO_REGISTER=true  # Allow new users to register via OIDC
ALLOW_LOCAL_AUTH=true  # Set to false to disable username/password login
```

#### Development Setup

```env
# Enable OIDC for development
OIDC_ENABLED=true

# Local development settings
OIDC_CLIENT_ID=dev-client-id
OIDC_CLIENT_SECRET=dev-client-secret
OIDC_ISSUER_URL=https://your-keycloak.company.com/auth/realms/readur
OIDC_REDIRECT_URI=http://localhost:8000/api/auth/oidc/callback
OIDC_AUTO_REGISTER=false  # Only allow existing users to login
ALLOW_LOCAL_AUTH=true  # Keep local auth for development
```

#### Docker Compose Setup

```yaml
version: '3.8'
services:
  readur:
    image: ghcr.io/readur/readur:main
    environment:
      # Core settings
      DATABASE_URL: postgresql://readur:readur@postgres:5432/readur
      
      # OIDC configuration
      OIDC_ENABLED: "true"
      OIDC_CLIENT_ID: "${OIDC_CLIENT_ID}"
      OIDC_CLIENT_SECRET: "${OIDC_CLIENT_SECRET}"
      OIDC_ISSUER_URL: "${OIDC_ISSUER_URL}"
      OIDC_REDIRECT_URI: "https://readur.company.com/api/auth/oidc/callback"
      OIDC_AUTO_REGISTER: "true"
    ports:
      - "8000:8000"
```

## Identity Provider Setup

### Google OAuth 2.0

1. **Create a Project** in [Google Cloud Console](https://console.cloud.google.com/)

2. **Enable Google+ API**:
   - Go to "APIs & Services" → "Library"
   - Search for "Google+ API" and enable it

3. **Create OAuth 2.0 Credentials**:
   - Go to "APIs & Services" → "Credentials"
   - Click "Create Credentials" → "OAuth 2.0 Client ID"
   - Application type: "Web application"
   - Name: "Readur Document Management"

4. **Configure Redirect URIs**:
   ```
   Authorized redirect URIs:
   https://your-readur-domain.com/api/auth/oidc/callback
   http://localhost:8000/api/auth/oidc/callback  (for development)
   ```

5. **Environment Variables**:
   ```env
   OIDC_ENABLED=true
   OIDC_CLIENT_ID=123456789-abcdefgh.apps.googleusercontent.com
   OIDC_CLIENT_SECRET=GOCSPX-your-secret-key
   OIDC_ISSUER_URL=https://accounts.google.com
   OIDC_REDIRECT_URI=https://your-readur-domain.com/api/auth/oidc/callback
   OIDC_AUTO_REGISTER=true
   ```

### Microsoft Azure AD

1. **Register an Application** in [Azure Portal](https://portal.azure.com/):
   - Go to "Azure Active Directory" → "App registrations"
   - Click "New registration"
   - Name: "Readur Document Management"
   - Supported account types: Choose based on your needs
   - Redirect URI: `https://your-readur-domain.com/api/auth/oidc/callback`

2. **Configure Authentication**:
   - In your app registration, go to "Authentication"
   - Add platform: "Web"
   - Add redirect URIs as needed
   - Enable "ID tokens" under "Implicit grant and hybrid flows"

3. **Create Client Secret**:
   - Go to "Certificates & secrets"
   - Click "New client secret"
   - Add description and choose expiration
   - **Copy the secret value immediately** (you won't see it again)

4. **Get Tenant Information**:
   - Note your Tenant ID from the "Overview" page
   - Issuer URL format: `https://login.microsoftonline.com/{tenant-id}/v2.0`

5. **Environment Variables**:
   ```env
   OIDC_ENABLED=true
   OIDC_CLIENT_ID=12345678-1234-1234-1234-123456789012
   OIDC_CLIENT_SECRET=your-client-secret
   OIDC_ISSUER_URL=https://login.microsoftonline.com/your-tenant-id/v2.0
   OIDC_REDIRECT_URI=https://your-readur-domain.com/api/auth/oidc/callback
   OIDC_AUTO_REGISTER=true
   ```

### Keycloak

1. **Create a Realm** (or use existing):
   - Access Keycloak admin console
   - Create or select a realm for Readur

2. **Create a Client**:
   - Go to "Clients" → "Create"
   - Client ID: `readur`
   - Client Protocol: `openid-connect`
   - Root URL: `https://your-readur-domain.com`

3. **Configure Client Settings**:
   - Access Type: `confidential`
   - Standard Flow Enabled: `ON`
   - Valid Redirect URIs: `https://your-readur-domain.com/api/auth/oidc/callback*`
   - Web Origins: `https://your-readur-domain.com`

4. **Get Client Secret**:
   - Go to "Credentials" tab
   - Copy the client secret

5. **Environment Variables**:
   ```env
   OIDC_ENABLED=true
   OIDC_CLIENT_ID=readur
   OIDC_CLIENT_SECRET=your-keycloak-client-secret
   OIDC_ISSUER_URL=https://keycloak.company.com/auth/realms/your-realm
   OIDC_REDIRECT_URI=https://your-readur-domain.com/api/auth/oidc/callback
   OIDC_AUTO_REGISTER=true
   ```

### Auth0

1. **Create an Application** in [Auth0 Dashboard](https://manage.auth0.com/):
   - Go to "Applications" → "Create Application"
   - Name: "Readur Document Management"
   - Application Type: "Regular Web Applications"

2. **Configure Settings**:
   - Allowed Callback URLs: `https://your-readur-domain.com/api/auth/oidc/callback`
   - Allowed Web Origins: `https://your-readur-domain.com`
   - Allowed Logout URLs: `https://your-readur-domain.com/login`

3. **Get Credentials**:
   - Note the Client ID and Client Secret from the "Settings" tab
   - Domain will be something like `your-app.auth0.com`

4. **Environment Variables**:
   ```env
   OIDC_ENABLED=true
   OIDC_CLIENT_ID=your-auth0-client-id
   OIDC_CLIENT_SECRET=your-auth0-client-secret
   OIDC_ISSUER_URL=https://your-app.auth0.com
   OIDC_REDIRECT_URI=https://your-readur-domain.com/api/auth/oidc/callback
   OIDC_AUTO_REGISTER=true
   ```

### Authentik

[Authentik](https://goauthentik.io/) is a self-hosted, open-source identity provider that's perfect for organizations wanting full control over their authentication.

1. **Create an Application** in Authentik Admin Interface:
   - Navigate to "Applications" → "Applications"
   - Click "Create" and choose "Create with Wizard"
   - Name: "Readur Document Management"
   - Slug: `readur` (or your preferred slug)

2. **Configure Provider**:
   - Provider type: Choose "OAuth2/OpenID Provider"
   - Authorization flow: "default-provider-authorization-implicit-consent"
   - Client type: "Confidential"
   - Redirect URIs: Add `https://your-readur-domain.com/api/auth/oidc/callback`
   - Scopes: Ensure `openid`, `email`, and `profile` are included

3. **Get Application Credentials**:
   - After creation, go to your application's "Provider" settings
   - Copy the Client ID (shown in the overview)
   - Copy the Client Secret (click "Copy" button)

4. **Configure Scopes and Claims** (Optional but recommended):
   - Go to "Customization" → "Property Mappings"
   - Ensure the following scope mappings exist and are enabled:
     - `openid` → `sub` claim
     - `email` → `email` claim
     - `profile` → `preferred_username` and `name` claims

5. **Get Issuer URL**:
   - The issuer URL format is: `https://your-authentik-domain.com/application/o/readur/`
   - Replace `readur` with your application's slug
   - Alternatively, use: `https://your-authentik-domain.com/application/o/<application-slug>/`

6. **Environment Variables**:
   ```env
   OIDC_ENABLED=true
   OIDC_CLIENT_ID=<your-authentik-client-id>
   OIDC_CLIENT_SECRET=<your-authentik-client-secret>
   OIDC_ISSUER_URL=https://your-authentik-domain.com/application/o/readur/
   OIDC_REDIRECT_URI=https://your-readur-domain.com/api/auth/oidc/callback
   OIDC_AUTO_REGISTER=true
   ```

7. **Testing Authentik Integration**:
   - Navigate to your Readur instance
   - Click "Sign in with OIDC"
   - You should be redirected to Authentik's login page
   - After authentication, you'll be redirected back to Readur

**Authentik-Specific Tips**:

- **User Attributes**: Authentik automatically provides `email`, `preferred_username`, and `name` in the OIDC claims
- **Group Mapping**: You can map Authentik groups to user attributes (future Readur feature will support role mapping)
- **Self-Service Portal**: Users can manage their Authentik profile at `https://your-authentik-domain.com/if/user/`
- **Email Verification**: If email verification is required in Authentik, ensure users verify their email before using Readur
- **Custom Branding**: Authentik allows you to customize the login page to match your organization's branding

**Docker Compose Example with Authentik**:

If you're running both Readur and Authentik with Docker Compose:

```yaml
version: '3.8'

services:
  authentik-server:
    image: ghcr.io/goauthentik/server:latest
    restart: unless-stopped
    command: server
    environment:
      AUTHENTIK_SECRET_KEY: your-secret-key-here
      AUTHENTIK_POSTGRESQL__HOST: postgresql
      AUTHENTIK_POSTGRESQL__NAME: authentik
      AUTHENTIK_POSTGRESQL__USER: authentik
      AUTHENTIK_POSTGRESQL__PASSWORD: authentik
    volumes:
      - ./media:/media
      - ./custom-templates:/templates
    ports:
      - "9000:9000"
      - "9443:9443"
    depends_on:
      - postgresql
      - redis

  readur:
    image: ghcr.io/readur/readur:main
    environment:
      DATABASE_URL: postgresql://readur:readur@postgres:5432/readur

      # Authentik OIDC Configuration
      OIDC_ENABLED: "true"
      OIDC_CLIENT_ID: "<from-authentik-application>"
      OIDC_CLIENT_SECRET: "<from-authentik-application>"
      OIDC_ISSUER_URL: "https://authentik.company.com/application/o/readur/"
      OIDC_REDIRECT_URI: "https://readur.company.com/api/auth/oidc/callback"
      OIDC_AUTO_REGISTER: "true"
    ports:
      - "8000:8000"
    depends_on:
      - postgres

  postgresql:
    image: postgres:17-alpine
    restart: unless-stopped
    environment:
      POSTGRES_PASSWORD: postgres
      POSTGRES_USER: postgres
    volumes:
      - database:/var/lib/postgresql/data
    # Create both databases
    command: >
      bash -c "
        docker-entrypoint.sh postgres &
        sleep 5
        psql -U postgres -c 'CREATE DATABASE authentik;'
        psql -U postgres -c 'CREATE DATABASE readur;'
        wait
      "

  redis:
    image: redis:alpine
    restart: unless-stopped

volumes:
  database:
  media:
```

**Troubleshooting Authentik**:

- **"Discovery failed"**: Verify the issuer URL includes the full path with application slug
- **"Invalid client"**: Double-check the Client ID matches exactly (no extra spaces)
- **"Redirect URI mismatch"**: Ensure the redirect URI in Authentik matches `OIDC_REDIRECT_URI` exactly
- **Email not syncing**: Check that the `email` scope is enabled in your Authentik application
- **Users not linking**: Verify emails match exactly between local users and Authentik user emails

### Generic OIDC Provider

For any OIDC-compliant provider:

1. **Register Your Application** with the provider
2. **Configure Redirect URI**: `https://your-readur-domain.com/api/auth/oidc/callback`
3. **Get Credentials**: Client ID, Client Secret, and Issuer URL
4. **Set Environment Variables**:
   ```env
   OIDC_ENABLED=true
   OIDC_CLIENT_ID=your-client-id
   OIDC_CLIENT_SECRET=your-client-secret
   OIDC_ISSUER_URL=https://your-provider.com
   OIDC_REDIRECT_URI=https://your-readur-domain.com/api/auth/oidc/callback
   OIDC_AUTO_REGISTER=true
   ```

## Testing the Setup

### 1. Verify Configuration Loading

When starting Readur, check the logs for OIDC configuration:

```
✅ OIDC_ENABLED: true (loaded from env)
✅ OIDC_CLIENT_ID: your-client-id (loaded from env)
✅ OIDC_CLIENT_SECRET: ***hidden*** (loaded from env, 32 chars)
✅ OIDC_ISSUER_URL: https://accounts.google.com (loaded from env)
✅ OIDC_REDIRECT_URI: https://your-domain.com/api/auth/oidc/callback (loaded from env)
```

### 2. Test Discovery Endpoint

Verify your provider's discovery endpoint works:

```bash
curl https://accounts.google.com/.well-known/openid-configuration
```

Should return JSON with `authorization_endpoint`, `token_endpoint`, and `userinfo_endpoint`.

### 3. Test Login Flow

Testing the complete authentication flow ensures everything is configured correctly. Navigate to your Readur login page where you should see the "Sign in with OIDC" button alongside the standard login form.

Click "Sign in with OIDC" to initiate the authentication process. You should be immediately redirected to your identity provider's login page. The URL should match your configured provider, and you should see your application's consent screen if this is the first time.

After successfully authenticating with your corporate credentials, the identity provider will redirect you back to Readur's callback URL. If everything is configured correctly, you'll land on the Readur dashboard as an authenticated user. Check that your username and email are correctly populated from the OIDC claims.

### 4. Check User Creation

Verify that OIDC users are created correctly in your system:

First, check your database for new users with `auth_provider = 'oidc'` to confirm the authentication method is properly recorded. You can run a query like `SELECT * FROM users WHERE auth_provider = 'oidc'` to see all OIDC-authenticated users.

Ensure the OIDC-specific fields are properly populated: `oidc_subject` should contain the unique identifier from your provider, `oidc_issuer` should match your configured issuer URL, and `oidc_email` should contain the user's email address from the identity provider.

Finally, verify that newly created users can actually access the dashboard and perform basic operations like uploading documents or searching. Check that their permissions are correctly set according to your default role configuration. You should also verify that the user's display name and other profile information were correctly extracted from the OIDC claims.

## User Experience

### First-Time Login

When a user signs in with OIDC for the first time:

1. User clicks "Sign in with OIDC"
2. Redirected to identity provider for authentication
3. After successful authentication, a new Readur account is created
4. User information is populated from OIDC claims:
   - **Username**: Derived from `preferred_username` or `email`
   - **Email**: From `email` claim
   - **OIDC Subject**: Unique identifier from `sub` claim
   - **Auth Provider**: Set to `oidc`

### Subsequent Logins

For returning users:

1. User clicks "Sign in with OIDC"
2. Readur matches the user by `oidc_subject` and `oidc_issuer`
3. User is automatically signed in without creating a duplicate account

### Email-Based User Syncing

Readur intelligently handles existing local users when they first log in via OIDC:

**Existing Local User with Matching Email**:
- When an OIDC user logs in with an email that matches an existing local user
- The OIDC identity is automatically linked to that existing account
- User retains all their documents, settings, and permissions
- The `auth_provider` field is updated to `oidc`
- Future logins can use OIDC (password still works if set)

**Example**: If you have a local user `john.doe@company.com`, and they log in via OIDC with the same email, their account is seamlessly upgraded to support OIDC authentication without creating a duplicate account.

### Auto-Registration Control

The `OIDC_AUTO_REGISTER` setting controls whether new users can self-register:

**When `OIDC_AUTO_REGISTER=true`**:
- New OIDC users are automatically created when they first log in
- Perfect for open environments where any company employee should get access
- Username is derived from OIDC claims (preferred_username or email)
- Users get the default "user" role

**When `OIDC_AUTO_REGISTER=false` (default)**:
- Only existing users (pre-created by admin or linked by email) can log in
- OIDC login attempts by unregistered users are rejected with HTTP 403
- Ideal for production environments requiring controlled access
- Admin must pre-create users before they can use OIDC

**Migration Strategy**: The default (`false`) is ideal for production. Have existing users log in to link accounts by email, then optionally enable `true` for new user auto-registration.

### Disabling Local Authentication

For OIDC-only deployments, you can disable local username/password authentication:

**When `ALLOW_LOCAL_AUTH=false`**:
- Local registration endpoint returns HTTP 403 Forbidden
- Local login endpoint returns HTTP 403 Forbidden
- Only OIDC authentication is available
- Perfect for enforcing SSO-only access
- Existing local users can still be linked via email when they use OIDC

**Security Benefits**:
- Single authentication method reduces attack surface
- Centralized password management through identity provider
- No need to manage password resets or policies in Readur
- Corporate password policies automatically apply

**Important Notes**:
- Ensure OIDC is working before disabling local auth
- At least one admin should test OIDC login first
- Cannot disable both OIDC and local auth (server will refuse to start)
- Recommended configuration for production SSO environments

**Example OIDC-Only Configuration**:
```env
# Enable OIDC
OIDC_ENABLED=true
OIDC_CLIENT_ID=your-client-id
OIDC_CLIENT_SECRET=your-client-secret
OIDC_ISSUER_URL=https://your-provider.com
OIDC_REDIRECT_URI=https://readur.company.com/api/auth/oidc/callback
OIDC_AUTO_REGISTER=true

# Disable local authentication
ALLOW_LOCAL_AUTH=false
```

### Mixed Authentication

When both authentication methods are enabled (`ALLOW_LOCAL_AUTH=true`):
- Local users can continue using username/password
- OIDC users can have both OIDC and password authentication
- Administrators can manage both types of users
- Email-based automatic account linking prevents duplicate accounts
- Users can choose their preferred login method

## Troubleshooting

### Common Issues

#### "OIDC client ID not configured"

**Problem**: OIDC environment variables not set correctly

**Solution**:
```bash
# Verify environment variables are set
echo $OIDC_ENABLED
echo $OIDC_CLIENT_ID
echo $OIDC_ISSUER_URL

# Check for typos in variable names
env | grep OIDC
```

#### "Failed to discover OIDC endpoints"

**Problem**: Cannot reach the OIDC discovery endpoint

**Solutions**:
- Verify `OIDC_ISSUER_URL` is correct
- Test connectivity: `curl https://your-issuer/.well-known/openid-configuration`
- Check firewall and network settings
- Ensure DNS resolution works

#### "Invalid redirect_uri"

**Problem**: Redirect URI mismatch between Readur and identity provider

**Solutions**:
- Verify `OIDC_REDIRECT_URI` matches exactly in both places
- Check for trailing slashes, HTTP vs HTTPS
- Ensure the provider allows your redirect URI

#### "Authentication failed: access_denied"

**Problem**: User denied access or provider restrictions

**Solutions**:
- Check user permissions in identity provider
- Verify the application is enabled for the user
- Review provider-specific restrictions

#### "Invalid authorization code"

**Problem**: Issues with the OAuth2 flow

**Solutions**:
- Check system clock synchronization
- Verify client secret is correct
- Look for network issues during token exchange

### Debug Mode

Enable detailed logging for OIDC troubleshooting:

```env
RUST_LOG=debug
```

This will show detailed information about:
- OIDC discovery process
- Token exchange
- User information retrieval
- Error details

### Testing with curl

Test the callback endpoint manually:

```bash
# Test the OIDC callback endpoint (after getting an auth code)
curl -X GET "https://your-readur-domain.com/api/auth/oidc/callback?code=AUTH_CODE&state=STATE"
```

## Security Considerations

### Production Deployment

1. **Use HTTPS**: Always use HTTPS in production
   ```env
   OIDC_REDIRECT_URI=https://readur.company.com/api/auth/oidc/callback
   ```

2. **Secure Client Secret**: Store client secrets securely
   - Use environment variables or secret management systems
   - Never commit secrets to version control
   - Rotate secrets regularly

3. **Validate Redirect URIs**: Ensure your identity provider only allows valid redirect URIs

4. **Network Security**: Restrict network access between Readur and identity provider

### User Management

1. **Account Mapping**: OIDC users are identified by `oidc_subject` + `oidc_issuer`
2. **No Password**: OIDC users don't have passwords in Readur
3. **User Deletion**: Deleting users from identity provider doesn't automatically remove them from Readur
4. **Role Management**: Configure user roles in Readur or map from OIDC claims

### Monitoring

Monitor OIDC authentication:

- Failed authentication attempts
- Token validation errors
- User creation patterns
- Provider availability

## Next Steps

After setting up OIDC:

1. **Test Thoroughly**: Test with different user accounts and scenarios
2. **User Training**: Inform users about the new login option
3. **Monitor Usage**: Track authentication patterns and issues
4. **Backup Strategy**: Ensure you can recover access if OIDC provider is unavailable
5. **Documentation**: Document your specific provider configuration for your team

For additional help:
- Review the [configuration guide](configuration.md) for general settings
- Check the [deployment guide](deployment.md) for production setup
- See the [user guide](user-guide.md) for end-user documentation