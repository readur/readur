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

Local authentication provides robust security through multiple layers of protection. **Secure Storage** ensures passwords are never stored in plain text, using bcrypt hashing with a cost factor of 12 to resist brute force attacks even if the database is compromised.

Authentication sessions are managed through **JWT Tokens** with 24-hour validity periods and secure signing algorithms, providing stateless authentication that scales well. The system supports **User Registration** for self-service account creation when enabled, streamlining onboarding for open deployments.

**Password Requirements** can be configured to enforce organizational policies, including minimum length, character complexity, and password history. Additionally, the system implements **Account Lockout** after failed login attempts and supports **Password Recovery** workflows via email.

#### Creating Local Users:

1. **Admin Creation** (via Settings):
   Administrators can create new user accounts directly through the settings interface. Navigate to Settings → Users (admin access required), then click "Add User" to open the creation form. Enter the new user's username, email address, and initial password, then assign the appropriate role (Admin or User). The system will validate the information and create the account immediately.

2. **Self Registration** (if enabled):
   When self-registration is enabled, new users can create their own accounts by visiting the registration page. They'll need to provide a username, email address, and secure password. Accounts created through self-registration are automatically assigned the default User role for security, though administrators can promote them later if needed.

2. **Self Registration** (if enabled):
   - Visit the registration page
   - Provide username, email, and password
   - Account created with default User role

### OIDC/SSO Authentication

OIDC (OpenID Connect) authentication integrates with enterprise identity providers for single sign-on.

#### Supported Features:

The OIDC implementation follows industry standards for maximum compatibility. **Standard OIDC Flow** uses the authorization code flow with PKCE (Proof Key for Code Exchange) for enhanced security, protecting against authorization code interception attacks.

**Automatic Discovery** simplifies configuration by reading provider settings from the standard `.well-known/openid-configuration` endpoint, eliminating manual configuration of authorization, token, and userinfo endpoints. The system handles **User Provisioning** seamlessly, automatically creating user accounts on first login with information from the identity provider.

**Identity Linking** ensures users are consistently identified by mapping OIDC identities to local user accounts using the subject claim and issuer URL. **Profile Sync** keeps user information current by updating email addresses and display names from the OIDC provider during each login. The system also supports **Group Mapping** for automatic role assignment and **Custom Claims** for extended user attributes.

#### Supported Providers:

Readur integrates with leading identity providers to support diverse organizational needs. **Microsoft Azure AD** provides comprehensive enterprise identity management with seamless integration into Microsoft 365 environments, supporting both cloud and hybrid deployments.

**Google Workspace** offers enterprise SSO for organizations using Google's productivity suite, with automatic user provisioning and group synchronization capabilities. **Okta** delivers a popular enterprise identity platform with extensive application integrations and advanced security features.

For flexible deployment options, **Auth0** provides a developer-friendly authentication platform with extensive customization options and multi-tenant support. **Keycloak** offers a powerful open-source identity management solution perfect for self-hosted environments. Additionally, **Generic OIDC** support ensures compatibility with any standards-compliant provider, including PingIdentity, OneLogin, and custom implementations.

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
- Password: Auto-generated on first startup (check container logs for "READUR ADMIN USER CREATED")

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

#### Modifying Users

To modify existing user accounts, first **Find the user** in the user list using the search function or scrolling through the user table. The interface supports filtering by role, authentication type, or activity status to quickly locate specific users.

**Click "Edit"** on the user row or select the user to open the modification interface. Here you can **Update information** including email addresses for communication, reset passwords when users forget credentials, modify roles to grant or revoke administrative privileges, and update usernames if organizational standards change.

Once you've made the necessary changes, **Save changes** to apply them immediately. The system will validate all modifications and update any active sessions if security-relevant changes were made. Note that some changes, like role modifications, may require the affected user to log out and back in to take full effect.

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

Fine-tune OCR processing to match your document types and quality requirements. **Image Enhancement** preprocessing improves OCR accuracy by applying contrast adjustment, noise reduction, and sharpening filters before text extraction, particularly beneficial for scanned documents.

**Auto-Rotation** detects and corrects document orientation automatically, ensuring text is properly aligned for optimal recognition accuracy. Users can set a **Confidence Threshold** to define the minimum acceptable confidence level for OCR results, with lower thresholds accepting more text but potentially including errors.

The **Processing Priority** setting determines your position in the OCR queue, with higher priority users getting faster processing during peak times. The system also offers **Deskewing** correction for slightly tilted scans, **Background Removal** for better text isolation, and **Resolution Enhancement** for low-quality images.

#### Search Preferences

**Display Settings:**

Customize how search results appear to match your workflow preferences. **Results Per Page** can be adjusted from 10 to 100 items, allowing you to balance between quick scanning and comprehensive result viewing based on your screen size and preference.

**Snippet Length** controls the amount of context shown around search matches, with options from brief 100-character excerpts to detailed 500-character passages that provide more context. The **Fuzzy Search Threshold** adjusts how tolerant searches are to spelling variations and OCR errors, essential for working with scanned documents.

You can enable or disable **Search History** based on privacy preferences and whether you want quick access to previous queries. Additionally, the interface provides **Highlight Colors** customization, **Preview Pane** sizing options, and **Result Grouping** preferences for organizing large result sets.

**Search Behavior:**
- **Default Sort Order**: Relevance, date, filename, size
- **Auto-Complete**: Enable search suggestions
- **Real-time Search**: Search as you type functionality

#### File Processing

**Upload Settings:**

Configure default behaviors for document uploads to streamline your workflow. **Default File Types** sets your preferred formats for the upload interface, pre-selecting common types you work with while hiding others to reduce clutter.

**Auto-OCR** configuration determines whether uploaded images and PDFs are automatically queued for text extraction, saving manual steps for documents that always need processing. The **Duplicate Handling** policy defines system behavior when uploading files that already exist - options include versioning, replacement, or rejection with user notification.

Personal **File Size Limits** can be set within system maximums, helping manage storage quotas and preventing accidental uploads of extremely large files. The system also supports **Upload Folders** for automatic organization, **Metadata Templates** for consistent tagging, and **Processing Rules** for document-type-specific workflows.

**Storage Preferences:**
- **Compression**: Enable compression for storage savings
- **Retention Period**: How long to keep documents (if configured)
- **Archive Behavior**: Automatic archiving of old documents

#### Interface Preferences

**Display Options:**

Personalize the visual interface to match your preferences and regional standards. **Theme** selection between light and dark modes reduces eye strain during extended use, with automatic switching based on time of day available as an option.

Set your **Timezone** to ensure all timestamps display in your local time, preventing confusion when coordinating with global teams or reviewing audit logs. **Date Format** preferences let you choose between various international formats (MM/DD/YYYY, DD/MM/YYYY, YYYY-MM-DD) matching your regional conventions.

The **Language** setting controls the interface language independently from OCR language settings, supporting multilingual users who process documents in different languages than their preferred interface. Additional display options include **Font Size** adjustments for accessibility, **Contrast Mode** for visual impairments, and **Compact View** for information-dense displays.

**Navigation:**
- **Default View**: List or grid view for document browser
- **Sidebar Collapsed**: Default sidebar state
- **Items Per Page**: Default pagination size

#### Notification Settings

**Notification Types:**

Stay informed about important events without constantly monitoring the system. **OCR Completion** notifications alert you when document processing finishes, particularly useful for large batches or priority documents that need immediate attention.

**Source Sync** notifications keep you updated on synchronization events from WebDAV, S3, or local folder sources, including new documents discovered and sync failures. **System Alerts** deliver important messages about maintenance windows, feature updates, or system-wide issues affecting service availability.

**Storage Warnings** proactively notify you when approaching storage quotas or when disk space is running low, giving time to clean up or request quota increases. The system also supports **Collaboration Notifications** for shared documents, **Security Alerts** for suspicious activity, and **Workflow Notifications** for document approval processes.

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

### Saving and Applying Settings

The settings system is designed for immediate feedback and minimal disruption. Start by modifying your preferences in the settings interface, where changes are validated in real-time with helpful hints for optimal configurations.

Once you're satisfied with your changes, click "Save Settings" to apply them. The system performs validation and saves your preferences to the database with automatic backup of previous settings. Most settings take effect immediately without requiring any additional action, allowing you to see results right away.

However, some settings - particularly those affecting authentication, session management, or fundamental interface changes - may require you to log out and back in for full application. The interface clearly indicates which settings require a session refresh, and you'll receive a notification if a re-login is needed for your changes to take complete effect.

## OIDC/SSO Integration

### Overview

OIDC integration allows users to authenticate using their corporate credentials without creating separate passwords for Readur.

### User Experience with OIDC

#### First-Time Login

#### First-Time Login

The first-time OIDC login experience is designed to be seamless and secure. When a new user clicks "Login with SSO" on the login page, they're immediately redirected to their corporate identity provider (such as Azure AD, Okta, or Google Workspace) where they're already familiar with the interface.

The user authenticates using their standard corporate credentials, potentially benefiting from single sign-on if they're already logged into other corporate applications. After successful authentication, Readur automatically creates a user account using information provided by the OIDC provider, including username, email, and display name.

The user is then logged in and can immediately start using Readur without any additional registration steps. Behind the scenes, the system has created their profile, assigned default permissions, and initialized their workspace. This automatic provisioning eliminates manual account creation while maintaining security through corporate identity verification.

#### Subsequent Logins

#### Subsequent Logins

Returning users experience an even smoother authentication process. Clicking "Login with SSO" triggers an automatic redirect to the identity provider, where the system recognizes the returning user.

If the user is already authenticated with their identity provider (common in corporate environments), single sign-on eliminates the need to re-enter credentials. The identity provider simply confirms the existing session and redirects back to Readur with authentication tokens.

This results in near-immediate access to Readur, often taking just seconds from click to dashboard. The seamless experience encourages regular use while maintaining security through centralized authentication management. The system also updates user profile information during each login, ensuring email addresses and display names stay synchronized with corporate directory changes.

### OIDC User Account Details

**Automatic Account Creation:**

OIDC authentication streamlines user provisioning through intelligent claim mapping. The **Username** is automatically derived from the OIDC `preferred_username` claim when available, falling back to the `sub` claim if needed, ensuring unique identification even when preferred usernames aren't provided.

The system extracts the user's **Email** from the standard `email` claim, using this for notifications and account recovery if needed. New OIDC users receive the default **Role** of "User" for security, though administrators can promote trusted users to admin roles after their initial login.

Accounts are clearly marked with **Auth Provider** as "OIDC" in the user management interface, helping administrators distinguish between local and federated accounts. The system also captures **Display Name** from the `name` claim, stores the **OIDC Issuer** for multi-provider support, and maintains **Last Login** timestamps for security auditing.

**Identity Mapping:**
- **OIDC Subject**: Unique identifier from identity provider
- **OIDC Issuer**: Identity provider URL
- **Linked Accounts**: Maps OIDC identity to Readur user

### Mixed Authentication Environments

Readur supports both local and OIDC users in the same installation:

### Mixed Authentication Environments

Readur's flexible authentication system supports both local and OIDC users simultaneously, providing operational flexibility. **Local Admin Accounts** remain available for initial system setup, emergency access when OIDC providers are unavailable, and system maintenance tasks that might require authentication during provider outages.

**OIDC User Accounts** serve regular enterprise users, providing seamless integration with corporate identity management, centralized password policies, and automatic deprovisioning when users leave the organization. This dual approach ensures business continuity while leveraging enterprise authentication.

**Role Management** works identically for both authentication types - administrators can promote OIDC users to admin roles just as easily as local users, ensuring organizational flexibility. The planned **Account Linking** feature will allow users to associate local and OIDC identities, enabling authentication fallback and migration scenarios. The system also maintains **Audit Trails** for both authentication types and supports **Conditional Access** policies based on authentication method.

### OIDC Configuration

See the detailed [OIDC Setup Guide](oidc-setup.md) for complete configuration instructions.

## Security Best Practices

### Password Security

**For Local Accounts:**

Protecting local accounts requires disciplined password management practices. **Use Strong Passwords** with a minimum of 12 characters combining uppercase and lowercase letters, numbers, and symbols. Consider using passphrases that are both memorable and secure, like "Coffee$Sunrise7Beach!Waves".

**Regular Rotation** of passwords reduces the risk window if credentials are compromised. Establish a rotation schedule based on account privilege levels - monthly for admin accounts, quarterly for regular users. The system can enforce password expiration policies to ensure compliance.

**Unique Passwords** are critical - never reuse passwords from other systems, as a breach elsewhere could compromise Readur access. Use a password manager to generate and store unique credentials for each system. For **Admin Passwords**, implement extra-strong requirements such as 16+ characters and consider using hardware tokens or certificate-based authentication for the highest privilege accounts. Additionally, implement **Password History** enforcement to prevent reuse and consider **Multi-factor Authentication** for sensitive accounts.

### JWT Token Security

**Token Management:**

JWT tokens require careful handling to maintain security while providing smooth user experiences. **Secure Storage** in browser localStorage provides a balance between security and usability, with tokens encrypted and accessible only to the application domain, though consider sessionStorage for higher security environments.

**Automatic Expiration** after 24 hours limits the exposure window if tokens are compromised, forcing regular re-authentication while avoiding excessive login prompts. The system validates expiration on every request, immediately rejecting expired tokens.

**Secure Transmission** over HTTPS is mandatory for production deployments, preventing token interception during network transmission. The system refuses to send tokens over unencrypted connections. Planned **Token Rotation** will implement refresh tokens for seamless session extension without re-authentication, reducing user friction while maintaining security. The system also implements **Token Revocation** capabilities and **Jti Tracking** to prevent token replay attacks.

### Access Control

**Role Management:**

Effective access control starts with the **Principle of Least Privilege** - grant users only the minimum permissions necessary for their job functions. Start with restrictive permissions and add capabilities as needed, rather than starting with broad access and trying to restrict later.

**Regular Review** of user roles and permissions ensures access remains appropriate as job responsibilities change. Schedule quarterly reviews where managers verify their team members' access levels, and automatically flag accounts that haven't been accessed in 30+ days.

Carefully control **Admin Accounts** by limiting their number to essential personnel only. Consider implementing temporary elevation for specific tasks rather than permanent admin rights. Implement **Account Deactivation** procedures for departed employees, including immediate access revocation, document ownership transfer, and audit log preservation. Additionally, maintain **Access Request Logs** for compliance and implement **Segregation of Duties** for sensitive operations.

### OIDC Security

**Provider Configuration:**

Secure OIDC integration requires careful attention to configuration details. **Use HTTPS** for all OIDC endpoints without exception - this includes authorization, token, userinfo, and JWKS endpoints. Configure certificate validation and reject any downgrade attempts to HTTP.

**Client Secret Protection** is critical since these secrets grant application-level access to your identity provider. Store secrets in environment variables or secure vaults, never in code repositories. Rotate secrets regularly and monitor for unauthorized usage.

**Scope Limitation** follows the principle of least privilege - request only the OIDC scopes necessary for Readur functionality (typically openid, profile, and email). Avoid requesting unnecessary scopes that could expose sensitive information. Implement **Token Validation** by properly verifying signatures, checking token expiration, validating issuer and audience claims, and ensuring nonce values match to prevent replay attacks. Also configure **PKCE** for public clients and implement **State Parameter** validation for CSRF protection.

### Monitoring and Auditing

**Access Monitoring:**

Comprehensive monitoring helps detect and respond to security incidents quickly. **Login Tracking** captures both successful and failed authentication attempts, including timestamp, IP address, and authentication method, helping identify brute force attacks or compromised credentials.

**Role Changes** require special attention - audit all administrator role assignments and revocations, including who made the change and when. Set up alerts for unexpected privilege escalations or role modifications outside normal business hours.

**Account Activity** monitoring tracks document access patterns to identify unusual behavior such as mass downloads, access from unexpected locations, or activity outside normal working hours. **Security Events** logging captures all authentication and authorization events in a tamper-resistant audit trail. This includes password changes, token generation, permission checks, and access denials. Additionally, implement **Geo-location Tracking** for access anomalies and **Session Analytics** for concurrent login detection.

## Troubleshooting

### Common Authentication Issues

#### Local Login Problems

**Symptom**: "Invalid username or password"
**Solutions**:

Resolving local login issues requires systematic troubleshooting. First, **Verify credentials** by carefully checking the username and password, paying attention to common issues like caps lock, leading/trailing spaces, or confusion between similar characters (0/O, 1/l/I).

**Account existence** should be confirmed in the user management interface - search for the user by email or username to ensure the account was created successfully and hasn't been accidentally deleted. Check if the username format matches system requirements.

**Password reset** by an administrator can resolve forgotten password issues quickly. Admins should generate a temporary password and require the user to change it on next login. For **Account status**, verify the account is active and not locked due to failed login attempts or administrative action. Check login attempt logs for specific error messages. Also verify **Database Connectivity** isn't causing authentication failures and check if **Case Sensitivity** in usernames is causing issues.

#### OIDC Login Problems

**Symptom**: OIDC login fails or redirects incorrectly
**Solutions**:

OIDC login failures often stem from configuration mismatches that can be systematically resolved. **Check OIDC configuration** by verifying the client ID matches exactly what's registered with your provider, ensure the client secret hasn't been rotated without updating Readur, and confirm the issuer URL includes the correct protocol and path.

**Redirect URI** must match exactly between Readur and the identity provider, including protocol (http vs https), hostname, port, and path. Even trailing slashes can cause mismatches. Check the provider's application settings for the registered redirect URIs.

**Provider status** should be verified by checking the provider's status page for outages, testing the discovery endpoint directly with curl, and confirming the provider's certificates are valid and not expired. For **Network connectivity**, verify Readur can reach OIDC endpoints by testing from the server (not just your browser), checking firewall rules for outbound HTTPS, and ensuring DNS resolution works for the provider's domains. Additionally, verify **TLS Version** compatibility and check **Time Synchronization** between servers.

#### JWT Token Issues

**Symptom**: "Invalid token" or frequent logouts
**Solutions**:

JWT token issues often manifest as unexpected logouts or authentication errors. **Check system time** synchronization between all servers - even a few minutes of drift can cause token validation failures. Use NTP to maintain accurate time across your infrastructure.

**JWT secret** configuration must be consistent across all application instances. Verify the JWT_SECRET environment variable is set correctly, contains a sufficiently random value, and hasn't been accidentally changed during deployment. The secret should be at least 32 characters long.

**Token expiration** after 24 hours is by design for security. If users report frequent logouts before this time, check for token validation issues, server restarts clearing in-memory state, or client-side storage problems. **Browser storage** issues can be resolved by clearing localStorage and cookies, then logging in fresh. Check browser console for storage quota errors or security restrictions. Also investigate **Clock Skew** tolerance settings and verify **Token Signature** algorithms match between signing and validation.

### User Management Issues

#### Cannot Create Users

**Symptom**: User creation fails
**Solutions**:

User creation failures can usually be traced to permission or data validation issues. **Admin permissions** are required for user creation - verify your account has the Admin role by checking your profile or asking another administrator to confirm your role assignment in the user management interface.

**Duplicate usernames** or email addresses will cause creation to fail with validation errors. Search existing users to ensure the username and email aren't already taken, including checking recently deleted users if soft-delete is enabled. The system enforces uniqueness on both fields.

**Database connectivity** issues can prevent user creation even with correct permissions. Check application logs for database connection errors, verify the database isn't in read-only mode, and ensure transaction logs haven't filled available disk space. For **Input validation**, confirm all required fields (username, email, password, role) are provided and meet validation requirements. Check for special characters that might need escaping and ensure email addresses are properly formatted. Also verify **Password Policy** compliance and check **Storage Quotas** haven't been exceeded.

#### User Settings Not Saving

**Symptom**: Settings changes don't persist
**Solutions**:

When user settings fail to save, investigate both client and server-side issues. **Check permissions** to ensure the user has the necessary rights to modify their settings - some organizations restrict certain settings to administrators only. Verify the user's session hasn't expired during editing.

**Database issues** can prevent settings from persisting even when the UI shows success. Verify the database has sufficient free space, check that the settings table isn't locked by long-running queries, and ensure the application has write permissions to the user preferences table. Review database logs for deadlocks or constraint violations.

**Browser issues** often cause settings to appear unsaved when they actually succeeded. Clear browser cache and cookies, try a different browser or incognito mode, and check for browser extensions that might interfere with form submission. **Network connectivity** problems during save operations can cause partial updates. Ensure a stable connection throughout the save process, check for proxy or firewall interference, and verify the request isn't timing out on slow connections. Also check **CSRF Token** validity and investigate **Session State** consistency.

### Role and Permission Issues

#### Users Cannot Access Features

**Symptom**: User reports missing functionality
**Solutions**:

Feature access issues typically stem from permission misconfigurations or incomplete role updates. **Check user role** in the user management interface to verify the correct role is assigned. Sometimes display issues show the wrong role while the database has the correct value - check both the UI and database directly.

**Permission scope** varies by role - confirm the specific feature is actually available to the user's assigned role by reviewing the role permission matrix. Some features might require additional flags beyond base role assignment. Check if custom permissions have been configured that override default role permissions.

**Session refresh** is often necessary after role changes because JWT tokens contain role information that isn't updated until re-authentication. Have the user log out completely and log back in to receive a fresh token with updated permissions. **Feature availability** at the system level should be verified - ensure the feature isn't disabled globally in configuration, check if it requires additional licensing or modules, and verify any feature flags are properly set. Also investigate **Caching Issues** that might show outdated permissions and check **Group Memberships** if using group-based permissions.

#### Admin Access Problems

**Symptom**: Admin cannot access management features
**Solutions**:

Admin access problems require careful verification of authentication and authorization chains. **Role verification** should start by confirming the Admin role in the user management interface, but also check the database directly to ensure there's no display discrepancy. Verify the role wasn't accidentally changed by reviewing audit logs.

**Token validity** is crucial for admin access - decode the JWT token (using jwt.io or similar tools) to ensure it contains the correct role claim. Check if the token was issued before a role change and still contains old permissions. Verify the token hasn't been tampered with by validating its signature.

**Database consistency** between the user's role and what's being authorized should be verified by checking for any pending database migrations that might affect permissions, ensuring role foreign keys are properly set, and confirming there are no orphaned permission records. A **Login refresh** often resolves token-related issues - have the user completely log out (clearing all sessions), clear browser storage to remove stale tokens, then log in fresh to receive updated credentials. Additionally, check **Permission Inheritance** and verify **Role Hierarchies** are properly configured.

### Performance Issues

#### Slow User Operations

**Symptom**: User management operations are slow
**Solutions**:

Slow user management operations can significantly impact administrator productivity. **Database performance** issues often cause slowdowns - analyze query execution plans for user-related queries, ensure indexes exist on frequently searched columns (username, email, role), and check for table bloat that might require maintenance.

**User count** impacts can be mitigated through pagination rather than loading all users at once. Implement lazy loading for user lists, add server-side filtering to reduce dataset size, and consider implementing virtual scrolling for large lists. Cache frequently accessed user data to reduce database load.

**Network latency** particularly affects OIDC operations where each user verification might require external API calls. Consider implementing batch operations for OIDC user updates, cache OIDC provider responses when appropriate, and use asynchronous processing for non-critical updates. **System resources** should be monitored during user operations - check if CPU spikes during user list generation, verify adequate memory for caching user data, and ensure disk I/O isn't bottlenecking database operations. Consider **Query Optimization** for complex permission checks and implement **Read Replicas** for user management queries.

## Next Steps

- Configure [OIDC integration](oidc-setup.md) for enterprise authentication
- Set up [sources](sources-guide.md) for document synchronization
- Review [security best practices](deployment.md#security-considerations)
- Explore [advanced search](advanced-search.md) capabilities
- Configure [labels and organization](labels-and-organization.md) for document management