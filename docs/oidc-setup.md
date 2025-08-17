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
  - [Generic OIDC Provider](#generic-oidc-provider)
- [Testing the Setup](#testing-the-setup)
- [User Experience](#user-experience)
- [Troubleshooting](#troubleshooting)
- [Security Considerations](#security-considerations)

## Overview

OIDC authentication in Readur provides:

- **Single Sign-On (SSO)**: Users can sign in with existing corporate accounts
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
| `OIDC_REDIRECT_URI` | ✅ | Callback URL for your Readur instance | `https://readur.company.com/auth/oidc/callback` |

### Example Configurations

#### Basic OIDC Setup

```env
# Enable OIDC
OIDC_ENABLED=true

# Provider settings (example for Google)
OIDC_CLIENT_ID=123456789-abcdefgh.apps.googleusercontent.com
OIDC_CLIENT_SECRET=GOCSPX-your-secret-key
OIDC_ISSUER_URL=https://accounts.google.com
OIDC_REDIRECT_URI=https://readur.company.com/auth/oidc/callback
```

#### Development Setup

```env
# Enable OIDC for development
OIDC_ENABLED=true

# Local development settings
OIDC_CLIENT_ID=dev-client-id
OIDC_CLIENT_SECRET=dev-client-secret
OIDC_ISSUER_URL=https://your-keycloak.company.com/auth/realms/readur
OIDC_REDIRECT_URI=http://localhost:8000/auth/oidc/callback
```

#### Docker Compose Setup

```yaml
version: '3.8'
services:
  readur:
    image: readur:latest
    environment:
      # Core settings
      DATABASE_URL: postgresql://readur:readur@postgres:5432/readur
      
      # OIDC configuration
      OIDC_ENABLED: "true"
      OIDC_CLIENT_ID: "${OIDC_CLIENT_ID}"
      OIDC_CLIENT_SECRET: "${OIDC_CLIENT_SECRET}"
      OIDC_ISSUER_URL: "${OIDC_ISSUER_URL}"
      OIDC_REDIRECT_URI: "https://readur.company.com/auth/oidc/callback"
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
   https://your-readur-domain.com/auth/oidc/callback
   http://localhost:8000/auth/oidc/callback  (for development)
   ```

5. **Environment Variables**:
   ```env
   OIDC_ENABLED=true
   OIDC_CLIENT_ID=123456789-abcdefgh.apps.googleusercontent.com
   OIDC_CLIENT_SECRET=GOCSPX-your-secret-key
   OIDC_ISSUER_URL=https://accounts.google.com
   OIDC_REDIRECT_URI=https://your-readur-domain.com/auth/oidc/callback
   ```

### Microsoft Azure AD

1. **Register an Application** in [Azure Portal](https://portal.azure.com/):
   - Go to "Azure Active Directory" → "App registrations"
   - Click "New registration"
   - Name: "Readur Document Management"
   - Supported account types: Choose based on your needs
   - Redirect URI: `https://your-readur-domain.com/auth/oidc/callback`

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
   OIDC_REDIRECT_URI=https://your-readur-domain.com/auth/oidc/callback
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
   - Valid Redirect URIs: `https://your-readur-domain.com/auth/oidc/callback*`
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
   OIDC_REDIRECT_URI=https://your-readur-domain.com/auth/oidc/callback
   ```

### Auth0

1. **Create an Application** in [Auth0 Dashboard](https://manage.auth0.com/):
   - Go to "Applications" → "Create Application"
   - Name: "Readur Document Management"
   - Application Type: "Regular Web Applications"

2. **Configure Settings**:
   - Allowed Callback URLs: `https://your-readur-domain.com/auth/oidc/callback`
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
   OIDC_REDIRECT_URI=https://your-readur-domain.com/auth/oidc/callback
   ```

### Generic OIDC Provider

For any OIDC-compliant provider:

1. **Register Your Application** with the provider
2. **Configure Redirect URI**: `https://your-readur-domain.com/auth/oidc/callback`
3. **Get Credentials**: Client ID, Client Secret, and Issuer URL
4. **Set Environment Variables**:
   ```env
   OIDC_ENABLED=true
   OIDC_CLIENT_ID=your-client-id
   OIDC_CLIENT_SECRET=your-client-secret
   OIDC_ISSUER_URL=https://your-provider.com
   OIDC_REDIRECT_URI=https://your-readur-domain.com/auth/oidc/callback
   ```

## Testing the Setup

### 1. Verify Configuration Loading

When starting Readur, check the logs for OIDC configuration:

```
✅ OIDC_ENABLED: true (loaded from env)
✅ OIDC_CLIENT_ID: your-client-id (loaded from env)
✅ OIDC_CLIENT_SECRET: ***hidden*** (loaded from env, 32 chars)
✅ OIDC_ISSUER_URL: https://accounts.google.com (loaded from env)
✅ OIDC_REDIRECT_URI: https://your-domain.com/auth/oidc/callback (loaded from env)
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

### Mixed Authentication

- Local users can continue using username/password
- OIDC users are created as separate accounts
- Administrators can manage both types of users
- No automatic account linking between local and OIDC accounts

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
   OIDC_REDIRECT_URI=https://readur.company.com/auth/oidc/callback
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