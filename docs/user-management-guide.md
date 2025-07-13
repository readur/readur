# User Management Guide

This comprehensive guide covers user administration, authentication, role-based access control, and user preferences in Readur.

## Table of Contents

- [Overview](#overview)
- [Authentication Methods](#authentication-methods)
- [User Roles and Permissions](#user-roles-and-permissions)
- [Admin User Management](#admin-user-management)
- [User Settings and Preferences](#user-settings-and-preferences)
- [OIDC/SSO Integration](#oidcsso-integration)
- [Security Best Practices](#security-best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

Readur provides a comprehensive user management system with support for both local authentication and enterprise SSO integration. The system features:

- **Dual Authentication**: Local accounts and OIDC/SSO support
- **Role-Based Access Control**: Admin and User roles with distinct permissions
- **User Preferences**: Extensive per-user configuration options
- **Enterprise Integration**: OIDC support for corporate identity providers
- **Security Features**: JWT tokens, bcrypt password hashing, and session management

## Authentication Methods

### Local Authentication

Local authentication uses traditional username/password combinations stored securely in Readur's database.

#### Features:
- **Secure Storage**: Passwords hashed with bcrypt (cost factor 12)
- **JWT Tokens**: 24-hour token validity with secure signing
- **User Registration**: Self-service account creation (if enabled)
- **Password Requirements**: Configurable complexity requirements

#### Creating Local Users:
1. **Admin Creation** (via Settings):
   - Navigate to Settings → Users (Admin only)
   - Click "Add User"
   - Enter username, email, and initial password
   - Assign user role (Admin or User)

2. **Self Registration** (if enabled):
   - Visit the registration page
   - Provide username, email, and password
   - Account created with default User role

### OIDC/SSO Authentication

OIDC (OpenID Connect) authentication integrates with enterprise identity providers for single sign-on.

#### Supported Features:
- **Standard OIDC Flow**: Authorization code flow with PKCE
- **Automatic Discovery**: Reads provider configuration from `.well-known/openid-configuration`
- **User Provisioning**: Automatic user creation on first login
- **Identity Linking**: Maps OIDC identities to local user accounts
- **Profile Sync**: Updates user information from OIDC provider

#### Supported Providers:
- **Microsoft Azure AD**: Enterprise identity management
- **Google Workspace**: Google's enterprise SSO
- **Okta**: Popular enterprise identity provider
- **Auth0**: Developer-friendly authentication platform
- **Keycloak**: Open-source identity management
- **Generic OIDC**: Any standards-compliant OIDC provider

See the [OIDC Setup Guide](oidc-setup.md) for detailed configuration instructions.

## User Roles and Permissions

### User Role

**Standard Users** have access to core document management functionality:

**Permissions:**
- ✅ Upload and manage own documents
- ✅ Search all documents (based on sharing settings)
- ✅ Configure personal settings and preferences
- ✅ Create and manage personal labels
- ✅ Use OCR processing features
- ✅ Access personal sources (WebDAV, local folders, S3)
- ✅ View personal notifications
- ❌ User management (cannot create/modify other users)
- ❌ System-wide settings or configuration
- ❌ Access to other users' private documents

### Admin Role

**Administrators** have full system access and user management capabilities:

**Additional Permissions:**
- ✅ **User Management**: Create, modify, and delete user accounts
- ✅ **System Settings**: Configure global system parameters
- ✅ **User Impersonation**: Access other users' documents (if needed)
- ✅ **System Monitoring**: View system health and performance metrics
- ✅ **Advanced Configuration**: OCR settings, source configurations
- ✅ **Security Management**: Token management, authentication settings

**Default Admin Account:**
- Username: `admin`
- Default Password: `readur2024` ⚠️ **Change immediately in production!**

## Admin User Management

### Accessing User Management

1. Log in as an administrator
2. Navigate to **Settings** → **Users**
3. The user management interface displays all system users

### User Management Operations

#### Creating Users

1. **Click "Add User"** in the Users section
2. **Fill out user information**:
   ```
   Username: john.doe
   Email: john.doe@company.com
   Password: [secure-password]
   Role: User (or Admin)
   ```
3. **Save** to create the account
4. **Notify the user** of their credentials

#### Modifying Users

1. **Find the user** in the user list
2. **Click "Edit"** or the user row
3. **Update information**:
   - Change email address
   - Reset password
   - Modify role (User ↔ Admin)
   - Update username (if needed)
4. **Save changes**

#### Deleting Users

1. **Select the user** to delete
2. **Click "Delete"** 
3. **Confirm deletion** (this action cannot be undone)

**Important Notes:**
- Users cannot delete their own accounts
- Deleting a user removes all their documents and settings
- Consider disabling instead of deleting for user retention

#### Bulk Operations

**Future Feature**: Bulk user operations for enterprise deployments:
- Bulk user import from CSV
- Bulk role changes
- Bulk user deactivation

### User Information Display

The user management interface shows:
- **Username and Email**: Primary identification
- **Role**: Current role assignment
- **Created Date**: Account creation timestamp
- **Last Login**: Recent activity indicator
- **Auth Provider**: Local or OIDC authentication method
- **Status**: Active/disabled status (future feature)

## User Settings and Preferences

### Personal Settings Access

Users can configure their preferences via:
1. **User Menu** → **Settings** (top-right corner)
2. **Settings Page** → **Personal** tab

### Settings Categories

#### OCR Preferences

**Language Settings:**
- **OCR Language**: Primary language for text recognition (25+ languages)
- **Fallback Languages**: Secondary languages for mixed documents
- **Auto-Detection**: Automatic language detection (if supported)

**Processing Options:**
- **Image Enhancement**: Enable preprocessing for better OCR results
- **Auto-Rotation**: Automatically rotate images for optimal text recognition
- **Confidence Threshold**: Minimum confidence level for OCR acceptance
- **Processing Priority**: User's OCR queue priority level

#### Search Preferences

**Display Settings:**
- **Results Per Page**: Number of search results to display (10-100)
- **Snippet Length**: Length of text previews in search results
- **Fuzzy Search Threshold**: Sensitivity for fuzzy/approximate matching
- **Search History**: Enable/disable search query history

**Search Behavior:**
- **Default Sort Order**: Relevance, date, filename, size
- **Auto-Complete**: Enable search suggestions
- **Real-time Search**: Search as you type functionality

#### File Processing

**Upload Settings:**
- **Default File Types**: Preferred file types for uploads
- **Auto-OCR**: Automatically queue uploads for OCR processing
- **Duplicate Handling**: How to handle duplicate file uploads
- **File Size Limits**: Personal file size restrictions

**Storage Preferences:**
- **Compression**: Enable compression for storage savings
- **Retention Period**: How long to keep documents (if configured)
- **Archive Behavior**: Automatic archiving of old documents

#### Interface Preferences

**Display Options:**
- **Theme**: Light/dark mode preference
- **Timezone**: Local timezone for timestamp display
- **Date Format**: Preferred date/time display format
- **Language**: Interface language (separate from OCR language)

**Navigation:**
- **Default View**: List or grid view for document browser
- **Sidebar Collapsed**: Default sidebar state
- **Items Per Page**: Default pagination size

#### Notification Settings

**Notification Types:**
- **OCR Completion**: Notify when document processing completes
- **Source Sync**: Notifications for source synchronization events
- **System Alerts**: Important system messages and warnings
- **Storage Warnings**: Alerts for storage space or quota issues

**Delivery Methods:**
- **In-App Notifications**: Browser notifications within Readur
- **Email Notifications**: Email delivery for important events (future)
- **Desktop Notifications**: Browser push notifications (future)

### Source-Specific Settings

**WebDAV Preferences:**
- **Connection Timeout**: How long to wait for WebDAV responses
- **Retry Attempts**: Number of retries for failed downloads
- **Sync Schedule**: Preferred automatic sync frequency

**Local Folder Settings:**
- **Watch Interval**: How often to scan local directories
- **File Permissions**: Permission handling for processed files
- **Symlink Handling**: Follow symbolic links during scans

### Saving and Applying Settings

1. **Modify preferences** in the settings interface
2. **Click "Save Settings"** to apply changes
3. **Settings take effect immediately** for most options
4. **Some settings** may require logout/login to fully apply

## OIDC/SSO Integration

### Overview

OIDC integration allows users to authenticate using their corporate credentials without creating separate passwords for Readur.

### User Experience with OIDC

#### First-Time Login

1. **User clicks "Login with SSO"** on login page
2. **Redirected to corporate identity provider** (e.g., Azure AD, Okta)
3. **User authenticates** with corporate credentials
4. **Readur creates user account automatically** with information from OIDC provider
5. **User is logged in** and can immediately start using Readur

#### Subsequent Logins

1. **Click "Login with SSO"**
2. **Automatic redirect** to identity provider
3. **Single sign-on** (may not require re-authentication)
4. **Immediate access** to Readur

### OIDC User Account Details

**Automatic Account Creation:**
- **Username**: Derived from OIDC `preferred_username` or `sub` claim
- **Email**: Uses OIDC `email` claim
- **Role**: Default "User" role (admins can promote later)
- **Auth Provider**: Marked as "OIDC" in user management

**Identity Mapping:**
- **OIDC Subject**: Unique identifier from identity provider
- **OIDC Issuer**: Identity provider URL
- **Linked Accounts**: Maps OIDC identity to Readur user

### Mixed Authentication Environments

Readur supports both local and OIDC users in the same installation:

- **Local Admin Accounts**: For initial setup and emergency access
- **OIDC User Accounts**: For regular enterprise users
- **Role Management**: Admins can promote OIDC users to admin role
- **Account Linking**: Future feature to link local and OIDC accounts

### OIDC Configuration

See the detailed [OIDC Setup Guide](oidc-setup.md) for complete configuration instructions.

## Security Best Practices

### Password Security

**For Local Accounts:**
1. **Use Strong Passwords**: Minimum 12 characters with mixed case, numbers, symbols
2. **Regular Rotation**: Change passwords periodically
3. **Unique Passwords**: Don't reuse passwords from other systems
4. **Admin Passwords**: Use extra-strong passwords for administrator accounts

### JWT Token Security

**Token Management:**
- **Secure Storage**: Tokens stored securely in browser localStorage
- **Automatic Expiration**: 24-hour token lifetime
- **Secure Transmission**: HTTPS required for production
- **Token Rotation**: Regular token refresh (future feature)

### Access Control

**Role Management:**
1. **Principle of Least Privilege**: Grant minimum necessary permissions
2. **Regular Review**: Periodically audit user roles and permissions
3. **Admin Accounts**: Limit number of administrator accounts
4. **Account Deactivation**: Disable accounts for departed users

### OIDC Security

**Provider Configuration:**
1. **Use HTTPS**: Ensure all OIDC endpoints use HTTPS
2. **Client Secret Protection**: Secure storage of OIDC client secrets
3. **Scope Limitation**: Request only necessary OIDC scopes
4. **Token Validation**: Proper verification of OIDC tokens

### Monitoring and Auditing

**Access Monitoring:**
- **Login Tracking**: Monitor successful and failed login attempts
- **Role Changes**: Audit administrator role assignments
- **Account Activity**: Track user document access patterns
- **Security Events**: Log authentication and authorization events

## Troubleshooting

### Common Authentication Issues

#### Local Login Problems

**Symptom**: "Invalid username or password"
**Solutions**:
1. **Verify credentials**: Check username/password carefully
2. **Account existence**: Confirm account exists in user management
3. **Password reset**: Admin can reset user password
4. **Account status**: Ensure account is active/enabled

#### OIDC Login Problems

**Symptom**: OIDC login fails or redirects incorrectly
**Solutions**:
1. **Check OIDC configuration**: Verify client ID, secret, and issuer URL
2. **Redirect URI**: Ensure redirect URI is registered with OIDC provider
3. **Provider status**: Confirm OIDC provider is operational
4. **Network connectivity**: Verify Readur can reach OIDC endpoints

#### JWT Token Issues

**Symptom**: "Invalid token" or frequent logouts
**Solutions**:
1. **Check system time**: Ensure server time is accurate
2. **JWT secret**: Verify JWT_SECRET environment variable
3. **Token expiration**: Tokens expire after 24 hours
4. **Browser storage**: Clear localStorage and re-login

### User Management Issues

#### Cannot Create Users

**Symptom**: User creation fails
**Solutions**:
1. **Admin permissions**: Ensure logged in as administrator
2. **Duplicate usernames**: Check for existing username/email
3. **Database connectivity**: Verify database connection
4. **Input validation**: Ensure all required fields are provided

#### User Settings Not Saving

**Symptom**: Settings changes don't persist
**Solutions**:
1. **Check permissions**: Ensure user has permission to modify settings
2. **Database issues**: Verify database write permissions
3. **Browser issues**: Try clearing browser cache
4. **Network connectivity**: Ensure stable connection during save

### Role and Permission Issues

#### Users Cannot Access Features

**Symptom**: User reports missing functionality
**Solutions**:
1. **Check user role**: Verify user has appropriate role assignment
2. **Permission scope**: Confirm feature is available to user role
3. **Session refresh**: User may need to logout/login after role change
4. **Feature availability**: Ensure feature is enabled in system configuration

#### Admin Access Problems

**Symptom**: Admin cannot access management features
**Solutions**:
1. **Role verification**: Confirm user has Admin role
2. **Token validity**: Ensure JWT token contains correct role information
3. **Database consistency**: Verify role is correctly stored in database
4. **Login refresh**: Try logging out and logging back in

### Performance Issues

#### Slow User Operations

**Symptom**: User management operations are slow
**Solutions**:
1. **Database performance**: Check database query performance
2. **User count**: Large user counts may require pagination
3. **Network latency**: OIDC operations may be affected by provider latency
4. **System resources**: Monitor CPU and memory usage

## Next Steps

- Configure [OIDC integration](oidc-setup.md) for enterprise authentication
- Set up [sources](sources-guide.md) for document synchronization
- Review [security best practices](deployment.md#security-considerations)
- Explore [advanced search](advanced-search.md) capabilities
- Configure [labels and organization](labels-and-organization.md) for document management