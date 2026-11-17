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
- **Email-Based User Syncing**: Optionally link existing local users to OIDC by verified email (`OIDC_LINK_EXISTING_BY_EMAIL`)
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
| `OIDC_ENABLED` | Yes | Enable OIDC authentication (`true`, `1`, `yes` or `on`) | `true` |
| `OIDC_CLIENT_ID` | Yes | OAuth2 client ID from your provider | `readur-app-client-id` |
| `OIDC_CLIENT_SECRET` | No | OAuth2 client secret from your provider. Leave it unset (or empty) for a public client; see [Public and Confidential Clients](#public-and-confidential-clients) | `very-secret-key` |
| `OIDC_ISSUER_URL` | Yes | The provider's issuer URL, exactly as its discovery document publishes it (a trailing `/` is ignored) | `https://accounts.google.com` |
| `OIDC_REDIRECT_URI` | Yes | Absolute callback URL of your Readur instance, ending in `/api/auth/oidc/callback`; register exactly this URL with the provider | `https://readur.example.com/api/auth/oidc/callback` |
| `OIDC_AUTO_REGISTER` | No | Create an account for an identity Readur does not know yet (default: `false`) | `true` or `false` |
| `OIDC_LINK_EXISTING_BY_EMAIL` | No | Link an OIDC login to an existing account with the same email (default: `false`). Only applies when the provider reports `email_verified: true` and the account is not linked to another identity | `true` or `false` |
| `ALLOW_LOCAL_AUTH` | No | Allow username/password authentication (default: `true`) | `true` or `false` |
| `PUBLIC_URL` | No | Public base URL of Readur, used for the post-login redirect back to the web UI. Defaults to the origin of `OIDC_REDIRECT_URI`. Request headers are not used to derive it | `https://readur.example.com` |

OIDC sign-in is available only when every required variable is set and Readur can fetch the provider's discovery document (`<OIDC_ISSUER_URL>/.well-known/openid-configuration`) at startup, and the issuer it publishes matches `OIDC_ISSUER_URL`. Otherwise the server still starts, logs `❌ Failed to initialize OIDC client: <reason>` followed by `❌ OIDC authentication will be disabled`, and offers local login only. The one combination that stops the server is OIDC disabled together with `ALLOW_LOCAL_AUTH=false`, since nobody could sign in.

After the provider redirects to `OIDC_REDIRECT_URI`, Readur sends the browser to `<PUBLIC_URL>/auth/callback#code=...`. The frontend exchanges this one-time code (valid for 60 seconds, usable once) for a session token at `POST /api/auth/oidc/exchange`; the token itself is never placed in the URL.

### Public and Confidential Clients

Readur always uses the authorization code flow with PKCE (`S256`) and requests the scopes `openid email profile`.

- **Confidential client** (`OIDC_CLIENT_SECRET` set): Readur authenticates to the token endpoint with HTTP Basic authentication (`client_secret_basic`). The provider may sign ID tokens with the client secret (`HS256`, `HS384`, `HS512`) or with a published key.
- **Public client** (`OIDC_CLIENT_SECRET` unset or empty): Readur sends only its `client_id` to the token endpoint and relies on PKCE. The startup log says `ℹ️  OIDC_CLIENT_SECRET not set: using public-client (PKCE-only) flow`. ID tokens must then be signed with one of the provider's published keys (RSA, RSA-PSS, ECDSA or EdDSA); HMAC-signed ID tokens are rejected because there is no secret to check them with.

Unsigned ID tokens are never accepted. The ID token's issuer, audience, expiry (with 60 seconds of leeway) and nonce are checked, and the userinfo endpoint, when the provider has one, may fill in claims the ID token lacks.

### Redirect URI and Cookies

`OIDC_REDIRECT_URI` must be an absolute URL that reaches Readur's `GET /api/auth/oidc/callback`, and must be registered with the provider exactly as configured (scheme, host, port, path and trailing slash). While a login is in progress Readur keeps its state in a cookie scoped to the directory of the redirect URI (so it also works when Readur is served under a path prefix such as `/readur/api/auth/oidc/callback`), marked `Secure` when the redirect URI uses `https://`.

### Example Configurations

#### Basic OIDC Setup

```env
# Enable OIDC
OIDC_ENABLED=true

# Provider settings (example for Google)
OIDC_CLIENT_ID=123456789-abcdefgh.apps.googleusercontent.com
OIDC_CLIENT_SECRET=GOCSPX-your-secret-key
OIDC_ISSUER_URL=https://accounts.google.com
OIDC_REDIRECT_URI=https://readur.example.com/api/auth/oidc/callback
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
OIDC_ISSUER_URL=https://keycloak.example.com/auth/realms/readur
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
      JWT_SECRET: "${JWT_SECRET}"   # optional; generate with: openssl rand -hex 32
      
      # OIDC configuration
      OIDC_ENABLED: "true"
      OIDC_CLIENT_ID: "${OIDC_CLIENT_ID}"
      OIDC_CLIENT_SECRET: "${OIDC_CLIENT_SECRET}"
      OIDC_ISSUER_URL: "${OIDC_ISSUER_URL}"
      OIDC_REDIRECT_URI: "https://readur.example.com/api/auth/oidc/callback"
      OIDC_AUTO_REGISTER: "true"
      PUBLIC_URL: "https://readur.example.com"
    ports:
      - "8000:8000"
```

## Identity Provider Setup

### Google OAuth 2.0

1. **Create a Project** in [Google Cloud Console](https://console.cloud.google.com/)

2. **Configure the OAuth consent screen** under "APIs & Services" → "OAuth consent screen"

3. **Create OAuth 2.0 Credentials**:
   - Go to "APIs & Services" → "Credentials"
   - Click "Create Credentials" → "OAuth 2.0 Client ID"
   - Application type: "Web application"
   - Name: "Readur Document Management"

4. **Configure Redirect URIs**:
   ```
   Authorized redirect URIs:
   https://readur.example.com/api/auth/oidc/callback
   http://localhost:8000/api/auth/oidc/callback  (for development)
   ```

5. **Environment Variables**:
   ```env
   OIDC_ENABLED=true
   OIDC_CLIENT_ID=123456789-abcdefgh.apps.googleusercontent.com
   OIDC_CLIENT_SECRET=GOCSPX-your-secret-key
   OIDC_ISSUER_URL=https://accounts.google.com
   OIDC_REDIRECT_URI=https://readur.example.com/api/auth/oidc/callback
   OIDC_AUTO_REGISTER=true
   ```

### Microsoft Azure AD

1. **Register an Application** in [Azure Portal](https://portal.azure.com/):
   - Go to "Azure Active Directory" → "App registrations"
   - Click "New registration"
   - Name: "Readur Document Management"
   - Supported account types: Choose based on your needs
   - Redirect URI: `https://readur.example.com/api/auth/oidc/callback`

2. **Configure Authentication**:
   - In your app registration, go to "Authentication"
   - Add platform: "Web"
   - Add redirect URIs as needed
   - Leave "Implicit grant and hybrid flows" off: Readur uses the authorization code flow with PKCE

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
   OIDC_REDIRECT_URI=https://readur.example.com/api/auth/oidc/callback
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
   - Root URL: `https://readur.example.com`

3. **Configure Client Settings**:
   - Access Type (Client authentication): `confidential` (on) to use a client secret, or `public` (off) for a public client without `OIDC_CLIENT_SECRET`
   - Standard Flow Enabled: `ON`
   - Valid Redirect URIs: `https://readur.example.com/api/auth/oidc/callback*`
   - Web Origins: `https://readur.example.com`

4. **Get Client Secret**:
   - Go to "Credentials" tab
   - Copy the client secret

5. **Environment Variables**:
   ```env
   OIDC_ENABLED=true
   OIDC_CLIENT_ID=readur
   OIDC_CLIENT_SECRET=your-keycloak-client-secret
   OIDC_ISSUER_URL=https://keycloak.example.com/auth/realms/your-realm
   OIDC_REDIRECT_URI=https://readur.example.com/api/auth/oidc/callback
   OIDC_AUTO_REGISTER=true
   ```

### Auth0

1. **Create an Application** in [Auth0 Dashboard](https://manage.auth0.com/):
   - Go to "Applications" → "Create Application"
   - Name: "Readur Document Management"
   - Application Type: "Regular Web Applications"

2. **Configure Settings**:
   - Allowed Callback URLs: `https://readur.example.com/api/auth/oidc/callback`
   - Allowed Web Origins: `https://readur.example.com`
   - Allowed Logout URLs: `https://readur.example.com/login`

3. **Get Credentials**:
   - Note the Client ID and Client Secret from the "Settings" tab
   - Domain will be something like `your-app.auth0.com`

4. **Environment Variables**:
   ```env
   OIDC_ENABLED=true
   OIDC_CLIENT_ID=your-auth0-client-id
   OIDC_CLIENT_SECRET=your-auth0-client-secret
   OIDC_ISSUER_URL=https://your-app.auth0.com
   OIDC_REDIRECT_URI=https://readur.example.com/api/auth/oidc/callback
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
   - Client type: "Confidential" (with `OIDC_CLIENT_SECRET`) or "Public" (without)
   - Redirect URIs: Add `https://readur.example.com/api/auth/oidc/callback`
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
   - The issuer URL format is: `https://authentik.example.com/application/o/readur/`
   - Replace `readur` with your application's slug
   - Alternatively, use: `https://authentik.example.com/application/o/<application-slug>/`

6. **Environment Variables**:
   ```env
   OIDC_ENABLED=true
   OIDC_CLIENT_ID=<your-authentik-client-id>
   OIDC_CLIENT_SECRET=<your-authentik-client-secret>
   OIDC_ISSUER_URL=https://authentik.example.com/application/o/readur/
   OIDC_REDIRECT_URI=https://readur.example.com/api/auth/oidc/callback
   OIDC_AUTO_REGISTER=true
   ```

7. **Testing Authentik Integration**:
   - Navigate to your Readur instance
   - Click "Sign in with OIDC"
   - You should be redirected to Authentik's login page
   - After authentication, you'll be redirected back to Readur

**Authentik-Specific Tips**:

- **User Attributes**: Authentik automatically provides `email`, `preferred_username`, and `name` in the OIDC claims
- **Roles**: Readur does not read group or role claims; every OIDC-created account gets the `user` role, and an administrator promotes accounts in Readur
- **Self-Service Portal**: Users can manage their Authentik profile at `https://authentik.example.com/if/user/`
- **Email Verification**: If email verification is required in Authentik, ensure users verify their email before using Readur
- **Custom Branding**: Authentik allows you to customize the login page to match your organization's branding

**Docker Compose Example with Authentik**:

Run Authentik as its own documentation describes (it needs its own database and secret key), and point Readur at it:

```yaml
services:
  readur:
    image: ghcr.io/readur/readur:main
    environment:
      DATABASE_URL: postgresql://readur:readur@postgres:5432/readur
      JWT_SECRET: "${JWT_SECRET}"   # optional; generate with: openssl rand -hex 32

      # Authentik OIDC Configuration
      OIDC_ENABLED: "true"
      OIDC_CLIENT_ID: "<from-authentik-application>"
      OIDC_CLIENT_SECRET: "<from-authentik-application>"
      OIDC_ISSUER_URL: "https://authentik.example.com/application/o/readur/"
      OIDC_REDIRECT_URI: "https://readur.example.com/api/auth/oidc/callback"
      OIDC_AUTO_REGISTER: "true"
      PUBLIC_URL: "https://readur.example.com"
    ports:
      - "8000:8000"
```

**Troubleshooting Authentik**:

- **"OIDC discovery failed" or "does not match OIDC_ISSUER_URL"**: Verify the issuer URL includes the full path with the application slug, exactly as Authentik shows it
- **"Invalid client"**: Double-check the Client ID matches exactly (no extra spaces)
- **"Redirect URI mismatch"**: Ensure the redirect URI in Authentik matches `OIDC_REDIRECT_URI` exactly
- **Email not syncing**: Check that the `email` scope is enabled in your Authentik application
- **Users not linking**: Linking requires `OIDC_LINK_EXISTING_BY_EMAIL=true`, an exact email match, and the user's email being verified in Authentik (`email_verified: true`)

### Generic OIDC Provider

For any OIDC-compliant provider:

1. **Register Your Application** with the provider
2. **Configure Redirect URI**: `https://readur.example.com/api/auth/oidc/callback`
3. **Get Credentials**: Client ID, Issuer URL and, for a confidential client, the Client Secret
4. **Set Environment Variables**:
   ```env
   OIDC_ENABLED=true
   OIDC_CLIENT_ID=your-client-id
   OIDC_CLIENT_SECRET=your-client-secret
   OIDC_ISSUER_URL=https://sso.example.com
   OIDC_REDIRECT_URI=https://readur.example.com/api/auth/oidc/callback
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
✅ OIDC_REDIRECT_URI: https://readur.example.com/api/auth/oidc/callback (loaded from env)
✅ OIDC client initialized successfully
```

A missing required variable shows as `❌ OIDC_CLIENT_ID is required when OIDC is enabled` (or the issuer URL or redirect URI), and an unreachable or mismatched provider as `❌ Failed to initialize OIDC client: ...`; in both cases OIDC stays disabled. `GET /api/auth/config` reports `"oidc_enabled": true` only once the client initialized.

### 2. Test Discovery Endpoint

Verify your provider's discovery endpoint works:

```bash
curl https://accounts.google.com/.well-known/openid-configuration
```

Should return JSON whose `issuer` equals your `OIDC_ISSUER_URL` (apart from a trailing `/`), with an `authorization_endpoint` and a `token_endpoint`. A `jwks_uri` is needed unless ID tokens are signed with the client secret, and a `userinfo_endpoint` is optional.

### 3. Test Login Flow

Testing the complete authentication flow ensures everything is configured correctly. Navigate to your Readur login page where you should see the "Sign in with OIDC" button alongside the standard login form.

Click "Sign in with OIDC" to initiate the authentication process. You should be immediately redirected to your identity provider's login page. The URL should match your configured provider, and you should see your application's consent screen if this is the first time.

After successfully authenticating with your corporate credentials, the identity provider will redirect you back to Readur's callback URL. If everything is configured correctly, you'll land on the Readur dashboard as an authenticated user. Check that your username and email are correctly populated from the OIDC claims.

### 4. Check User Creation

Verify that OIDC users are created correctly in your system:

First, check your database for users with `auth_provider = 'oidc'` to confirm the authentication method is recorded. You can run a query like `SELECT username, email, oidc_subject, oidc_issuer FROM users WHERE auth_provider = 'oidc'` to see all OIDC-linked accounts.

Ensure the OIDC-specific fields are populated: `oidc_subject` holds the `sub` claim, `oidc_issuer` the issuer from the provider's discovery document, and `oidc_email` the email the provider asserted.

Finally, verify that newly created users can access the dashboard and perform basic operations like uploading documents or searching. New OIDC accounts always get the `user` role.

## User Experience

### First-Time Login

When a user signs in with OIDC for the first time:

1. User clicks "Sign in with OIDC"
2. Redirected to identity provider for authentication
3. After successful authentication, Readur looks for an account (see [Email-Based User Syncing](#email-based-user-syncing)) and, if there is none and `OIDC_AUTO_REGISTER=true`, creates one
4. A created account is populated from the OIDC claims:
   - **Username**: The `preferred_username` claim, else the `email` claim, when it is a valid username that is not taken; otherwise `oidc_` followed by 12 hexadecimal characters derived from the issuer and subject. An OIDC login never attaches to an existing account by username
   - **Email**: From the `email` claim, or `<username>@oidc.invalid` when the provider sends none
   - **OIDC Subject**: Unique identifier from the `sub` claim
   - **Role**: `user`
   - **Password**: None, so the account can only sign in through OIDC

### Subsequent Logins

For returning users:

1. User clicks "Sign in with OIDC"
2. Readur matches the user by `oidc_subject` and `oidc_issuer`
3. User is automatically signed in without creating a duplicate account

### Email-Based User Syncing

Linking an OIDC identity to an existing local account by email is controlled by `OIDC_LINK_EXISTING_BY_EMAIL` (default: `false`).

**When `OIDC_LINK_EXISTING_BY_EMAIL=false` (default)**:
- An OIDC login whose email matches an existing account that is not already linked to that OIDC identity is refused: the browser returns to Readur's login page with the error `no_account`
- No duplicate account is created
- An administrator can resolve this by changing or removing the existing account, or by enabling linking

**When `OIDC_LINK_EXISTING_BY_EMAIL=true`**:
- The OIDC identity is linked to the existing account only if the identity provider reports `email_verified: true` and the account is not already linked to another OIDC identity
- User retains all their documents, settings, and permissions
- The `auth_provider` field is updated to `oidc`
- Future logins can use OIDC, and the account's password keeps working for local login while `ALLOW_LOCAL_AUTH` is on

**Example**: With linking enabled, if you have a local user `john.doe@example.com` and they log in via OIDC with the same, verified email, their account is upgraded to support OIDC authentication without creating a duplicate account.

### Auto-Registration Control

The `OIDC_AUTO_REGISTER` setting controls whether new users can self-register:

**When `OIDC_AUTO_REGISTER=true`**:
- New OIDC users are automatically created when they first log in
- Perfect for open environments where any company employee should get access
- Username is derived from OIDC claims (preferred_username or email)
- Users get the default "user" role

**When `OIDC_AUTO_REGISTER=false` (default)**:
- Only existing users (pre-created by admin or linked by email) can log in
- OIDC login attempts by unknown users are refused with the error `no_account` on the login page
- Ideal for production environments requiring controlled access
- Admin must pre-create users before they can use OIDC

**Migration Strategy**: The default (`false`) is ideal for production. To move existing local users to OIDC, set `OIDC_LINK_EXISTING_BY_EMAIL=true` and have them log in to link accounts by verified email, then optionally enable `OIDC_AUTO_REGISTER=true` for new user auto-registration.

### Disabling Local Authentication

For OIDC-only deployments, you can disable local username/password authentication:

**When `ALLOW_LOCAL_AUTH=false`**:
- Local registration endpoint returns HTTP 403 Forbidden
- Local login endpoint returns HTTP 403 Forbidden
- Only OIDC authentication is available
- Perfect for enforcing SSO-only access
- Existing local users can still be linked via verified email when they use OIDC, if `OIDC_LINK_EXISTING_BY_EMAIL=true`

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
OIDC_ISSUER_URL=https://sso.example.com
OIDC_REDIRECT_URI=https://readur.example.com/api/auth/oidc/callback
OIDC_AUTO_REGISTER=true

# Disable local authentication
ALLOW_LOCAL_AUTH=false
```

### Mixed Authentication

When both authentication methods are enabled (`ALLOW_LOCAL_AUTH=true`):
- Local users can continue using username/password
- Accounts linked to OIDC by email keep their password; accounts created by OIDC have none
- Administrators can manage both types of users
- Email-based account linking (with `OIDC_LINK_EXISTING_BY_EMAIL=true`) avoids duplicate accounts
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

#### "Failed to fetch OIDC discovery document" or "OIDC discovery failed with status"

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

#### "does not match OIDC_ISSUER_URL"

**Problem**: The issuer in the provider's discovery document differs from `OIDC_ISSUER_URL`

**Solution**: Copy the `issuer` value from `<OIDC_ISSUER_URL>/.well-known/openid-configuration` into `OIDC_ISSUER_URL`.

#### "HMAC-signed ID token but no client secret configured"

**Problem**: The provider signs ID tokens with the client secret, but Readur runs as a public client

**Solution**: Set `OIDC_CLIENT_SECRET`, or configure the provider to sign ID tokens with an asymmetric key (for example `RS256`).

#### Login page shows `no_account`

**Problem**: The identity is not linked to an account, and Readur may not create or link one

**Solution**: Enable `OIDC_AUTO_REGISTER`, or `OIDC_LINK_EXISTING_BY_EMAIL` for accounts whose email the provider verifies, or have an administrator create the account first. The other error codes the login page can show are `provider_error`, `invalid_state` (the login took too long or the state cookie was lost), `auth_failed`, `account_disabled` and `server_error`.

### Testing with curl

```bash
# Is OIDC active? ("oidc_enabled": true once the client initialized)
curl https://readur.example.com/api/auth/config

# Does the login start? Expect 302 with a Location header pointing at your provider
curl -i https://readur.example.com/api/auth/oidc/login
```

## Security Considerations

### Production Deployment

1. **Use HTTPS**: Always use HTTPS in production
   ```env
   OIDC_REDIRECT_URI=https://readur.example.com/api/auth/oidc/callback
   ```

2. **Secure Client Secret**: Store client secrets securely
   - Use environment variables or secret management systems
   - Never commit secrets to version control
   - Rotate secrets regularly

3. **Validate Redirect URIs**: Ensure your identity provider only allows valid redirect URIs

4. **Network Security**: Restrict network access between Readur and identity provider

### User Management

1. **Account Mapping**: OIDC users are identified by `oidc_subject` + `oidc_issuer`
2. **No Password**: Accounts created through OIDC have no password in Readur
3. **User Deletion**: Deleting users from identity provider doesn't automatically remove them from Readur
4. **Role Management**: Roles are managed in Readur; group and role claims from the provider are not used

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