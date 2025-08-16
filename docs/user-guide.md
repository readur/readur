# User Guide

A comprehensive guide to using Readur's features for document management, OCR processing, and search.

## Table of Contents

- [Getting Started](#getting-started)
- [Supported File Types](#supported-file-types)
- [Using the Interface](#using-the-interface)
  - [Dashboard](#dashboard)
  - [Document Management](#document-management)
  - [Advanced Search](#advanced-search)
  - [Sources and Synchronization](#sources-and-synchronization)
- [Document Upload](#document-upload)
- [OCR Processing](#ocr-processing)
- [Search Features](#search-features)
- [Labels and Organization](#labels-and-organization)
- [User Management](#user-management)
- [User Settings](#user-settings)
- [Tips for Best Results](#tips-for-best-results)

## Getting Started

1. **First Login**: 
   - Navigate to `http://localhost:8000` (or your configured URL)
   - Use the default admin credentials (username: `admin`, password: `readur2024`)
   - **Important**: Change the default password immediately

2. **Initial Setup**:
   - Configure your user preferences
   - Set OCR language if different from English
   - Adjust search and display settings

3. **Quick Start**:
   - Upload your first document using drag-and-drop or the upload button
   - Wait for OCR processing to complete
   - Search for content within your documents

## Supported File Types

| Type | Extensions | OCR Support | Notes |
|------|-----------|-------------|-------|
| **PDF** | `.pdf` | ✅ | Text extraction + OCR for scanned pages |
| **Images** | `.png`, `.jpg`, `.jpeg`, `.tiff`, `.bmp`, `.gif` | ✅ | Full OCR text extraction |
| **Text** | `.txt`, `.rtf` | ❌ | Direct text indexing |
| **Office** | `.doc`, `.docx` | ⚠️ | Limited support |

## Using the Interface

### Dashboard

The dashboard provides an overview of your document system:

- **Document Statistics**: 
  - Total documents in the system
  - Storage usage breakdown
  - OCR processing status
  - Recent activity timeline

- **Quick Actions**:
  - Upload new documents
  - Quick search bar
  - Access to recent documents
  - System notifications

### Document Management

#### List/Grid View
- **List View**: Detailed document information in a table format
- **Grid View**: Visual thumbnails for quick browsing
- Toggle between views using the view selector in the top toolbar

#### Sorting Options
- Upload date (newest/oldest first)
- File name (A-Z/Z-A)
- File size (largest/smallest)
- Document type
- OCR status

#### Filtering
- By file type (PDF, images, text)
- By OCR status (completed, pending, failed)
- By date range
- By tags
- By source (uploaded, watched folder)

#### Bulk Actions
1. Select multiple documents using checkboxes
2. Available bulk actions:
   - Delete selected documents
   - Add/remove tags
   - Export document list
   - Reprocess OCR

### Advanced Search

Readur offers powerful search capabilities:

#### Full-Text Search
- Search within document content
- Automatic stemming and fuzzy matching
- Phrase search with quotes: `"exact phrase"`
- Exclude terms with minus: `-excluded`

#### Search Filters
- **Date Range**: Find documents from specific time periods
- **File Type**: Limit search to specific formats
- **File Size**: Filter by document size
- **OCR Status**: Only search processed documents
- **Tags**: Search within tagged documents

#### Search Syntax
```
invoice 2024              # Find documents with both terms
"quarterly report"        # Exact phrase search
invoice -draft           # Exclude drafts
tag:important invoice    # Search within tagged documents
type:pdf contract        # Search only PDFs
```

### Sources and Synchronization

Readur's Sources feature provides automated document ingestion from multiple external storage systems:

1. **Multi-Protocol Support**: WebDAV, Local Folders, and S3-compatible storage
2. **Non-destructive**: Source files remain untouched in their original locations
3. **Automated Syncing**: Scheduled synchronization with configurable intervals
4. **Health Monitoring**: Proactive monitoring and validation of source connections
5. **Intelligent Processing**: Duplicate detection, incremental syncs, and OCR integration

#### Supported Source Types

- **WebDAV Sources**: Nextcloud, ownCloud, generic WebDAV servers
- **Local Folder Sources**: Local filesystem directories and network mounts
- **S3 Sources**: Amazon S3 and S3-compatible storage (MinIO, DigitalOcean Spaces)

#### Setting Up Sources
1. Navigate to Settings → Sources
2. Click "Add Source" and select source type
3. Configure connection details and credentials
4. Test connection and configure sync settings
5. Set up folders to monitor and sync schedule

> 📖 **For comprehensive source configuration**, see the [Sources Guide](sources-guide.md)

## Document Upload

### Manual Upload
1. Click the upload button or drag files to the upload area
2. Select one or multiple files
3. Add tags during upload (optional)
4. Click "Upload" to start processing

### Drag and Drop
- Drag files directly from your file manager
- Drop anywhere on the document list page
- Multiple files can be dropped at once

### Upload Limits
- Maximum file size: Configurable (default 50MB)
- Supported formats: See [Supported File Types](#supported-file-types)
- Batch upload: Up to 100 files at once

## OCR Processing

### Automatic OCR
- Starts automatically after upload
- Processes documents in background
- Priority queue for smaller files

### OCR Settings
- **Language**: Select from 100+ languages
- **Preprocessing**: Enable image enhancement
- **Auto-rotation**: Correct document orientation
- **Quality**: Balance between speed and accuracy

### OCR Status Indicators
- 🟢 **Completed**: Full text extracted
- 🟡 **Processing**: OCR in progress
- 🔴 **Failed**: Error during processing
- ⚪ **Pending**: Waiting in queue

## Search Features

Readur provides powerful search capabilities with multiple modes and advanced filtering options.

### Search Modes

- **Simple Search**: General purpose searching with automatic stemming and fuzzy matching
- **Phrase Search**: Find exact phrases using quotes (e.g., `"quarterly report"`)
- **Fuzzy Search**: Handle typos and OCR errors with approximate matching (e.g., `invoice~`)
- **Boolean Search**: Complex queries with AND, OR, NOT operators

### Search Interface

#### Quick Search
- Available in the header on all pages
- Instant results as you type
- Shows top 5 matches with snippets
- Real-time suggestions

#### Advanced Search Page
- Full search interface with all filters
- Multiple search modes selector
- Comprehensive filtering options
- Export search results
- Save frequently used searches
- Search history and analytics

### Advanced Filtering

- **File Types**: Filter by PDF, images, documents, etc.
- **Date Ranges**: Search within specific time periods
- **Labels**: Filter by document tags and categories
- **Sources**: Search within specific sync sources
- **File Size**: Filter by document size ranges
- **OCR Status**: Filter by text extraction status

### Search Tips
1. Use quotes for exact phrases: `"project status"`
2. Combine text search with filters for precision
3. Use wildcards: `proj*` matches project, projects, projection
4. Search specific fields: `filename:report`, `label:urgent`
5. Use boolean logic: `(budget OR financial) AND 2024`

> 🔍 **For detailed search techniques**, see the [Advanced Search Guide](advanced-search.md)

## Labels and Organization

Readur's labeling system provides comprehensive document organization and categorization capabilities.

### Label Types

- **User Labels**: Custom labels created and managed by users with full control
- **System Labels**: Automatic labels generated by Readur (OCR status, file type, etc.)
- **Color Coding**: Visual identification with customizable label colors
- **Hierarchical Structure**: Organize labels in categories and subcategories

### Creating and Managing Labels

#### Creating Labels
1. **Via Settings**: Go to Settings → Labels and click "Create Label"
2. **During Upload**: Add labels while uploading documents
3. **Document Details**: Add labels directly from document pages
4. **Bulk Operations**: Create and assign labels to multiple documents

#### Label Operations
- **Rename**: Change label names (updates all documents)
- **Merge**: Combine similar labels into one
- **Color Management**: Customize label colors for visual organization
- **Bulk Assignment**: Apply labels to multiple documents at once

### Organization Strategies

#### Category-Based Organization
- **Projects**: "Project Alpha", "Q1 Budget", "Infrastructure"
- **Departments**: "HR", "Finance", "Legal", "Marketing"
- **Document Types**: "Invoices", "Contracts", "Reports", "Policies"
- **Status**: "Draft", "Final", "Approved", "Archived"

#### Time-Based Organization
- **Fiscal Periods**: "Q1 2024", "FY2024", "Annual Review"
- **Project Phases**: "Planning", "Implementation", "Review"
- **Event-Based**: "Pre-Launch", "Launch", "Post-Launch"

### Smart Collections
Create saved searches that automatically include documents with specific labels:
- **Active Projects**: Documents with current project labels
- **Pending Review**: Documents labeled for review
- **High Priority**: Documents with urgent or critical labels

> 🏷️ **For comprehensive labeling strategies**, see the [Labels and Organization Guide](labels-and-organization.md)

## User Management

Readur provides comprehensive user management with support for both local authentication and enterprise SSO integration.

### Authentication Methods

#### Local Authentication
- **Traditional Login**: Username and password authentication
- **Secure Storage**: Passwords hashed with bcrypt for security
- **Self Registration**: Users can create their own accounts (if enabled)

#### OIDC/SSO Authentication
- **Enterprise Integration**: Single Sign-On with corporate identity providers
- **Supported Providers**: Microsoft Azure AD, Google Workspace, Okta, Auth0, Keycloak
- **Automatic Provisioning**: User accounts created automatically on first login
- **Seamless Experience**: Users authenticate with existing corporate credentials

### User Roles and Permissions

#### User Role
Standard users with access to core document management functionality:
- Upload and manage documents
- Search and view documents
- Configure personal settings
- Create and manage labels
- Set up personal sources

#### Admin Role
Administrators with full system access and user management capabilities:
- **User Management**: Create, modify, and delete user accounts
- **System Settings**: Configure global system parameters  
- **Role Management**: Assign and modify user roles
- **System Monitoring**: View system health and performance metrics

### Administrative Features

Administrators can access user management via Settings → Users:
- **Create Users**: Add new user accounts with role assignment
- **Modify Users**: Update user information, roles, and passwords
- **User Overview**: View all users with creation dates and roles
- **Authentication Methods**: Manage both local and OIDC users
- **Bulk Operations**: Perform operations on multiple users

### Mixed Authentication Environments

Readur supports both local and OIDC users in the same installation:
- Local admin accounts for system management
- OIDC user accounts for regular enterprise users
- Flexible role assignment regardless of authentication method

> 👥 **For detailed user administration**, see the [User Management Guide](user-management-guide.md)
> 🔐 **For OIDC configuration**, see the [OIDC Setup Guide](oidc-setup.md)

## User Settings

### Personal Preferences
- **Display**: List/grid default view
- **Language**: Interface language
- **Time Zone**: For accurate timestamps
- **Notifications**: Email/in-app alerts

### OCR Preferences
- Default OCR language
- Processing priority
- Image preprocessing options
- Batch size limits

### Search Settings
- Results per page
- Default sort order
- Snippet length
- Fuzzy search threshold

## Tips for Best Results

### OCR Quality
1. **Higher Resolution**: 300+ DPI produces better OCR results
2. **Clean Scans**: Avoid skewed or dirty documents
3. **Good Lighting**: For photo captures, ensure even lighting
4. **Text Contrast**: Black text on white background works best

### File Organization
1. **Consistent Naming**: Use descriptive, consistent file names
2. **Regular Uploads**: Don't let documents pile up
3. **Use Tags**: Tag documents immediately after upload
4. **Folder Structure**: Organize watch folders logically

### Search Optimization
1. **Use Filters**: Combine text search with filters
2. **Save Searches**: Save frequently used search queries
3. **Learn Syntax**: Master search operators for better results
4. **Index Regularly**: Ensure all documents are processed

### Performance Tips
1. **Batch Processing**: Upload similar documents together
2. **Off-Peak Hours**: Schedule large uploads during low-usage times
3. **Monitor Queue**: Check OCR queue status regularly
4. **Clean Up**: Remove outdated documents periodically

## Troubleshooting

### Common Issues

**OCR Not Starting**
- Check file size limits
- Verify supported file format
- Ensure OCR service is running

**Search Not Finding Documents**
- Confirm OCR completed successfully
- Check search syntax
- Try broader search terms

**Slow Performance**
- Review concurrent OCR job settings
- Check system resources
- Consider increasing memory limits

## Next Steps

### Explore Advanced Features

- [🔗 Sources Guide](sources-guide.md)  
  Set up WebDAV, Local Folder, and S3 synchronization
  
- [🔎 Advanced Search](advanced-search.md)  
  Master search modes, syntax, and optimization
  
- [🏷️ Labels & Organization](labels-and-organization.md)  
  Implement effective document organization
  
- [👥 User Management](user-management-guide.md)  
  Configure authentication and user administration
  
- [🔐 OIDC Setup](oidc-setup.md)  
  Integrate with enterprise identity providers

### System Administration

- [📦 Installation Guide](installation.md)  
  Full installation and setup instructions
  
- [🔧 Configuration](configuration.md)  
  Environment variables and advanced configuration
  
- [🚀 Deployment Guide](deployment.md)  
  Production deployment with SSL and monitoring
  
- [📁 Watch Folder Guide](WATCH_FOLDER.md)  
  Legacy folder watching setup

### Development and Integration

- [🔌 API Reference](api-reference.md)  
  REST API for automation and integration
  
- [🏗️ Developer Documentation](dev/README.md)  
  Architecture and development setup
  
- [🔍 OCR Optimization](dev/OCR_OPTIMIZATION_GUIDE.md)  
  Improve OCR performance
  
- [📊 Queue Architecture](dev/QUEUE_IMPROVEMENTS.md)  
  Background processing optimization