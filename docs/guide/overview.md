# Readur User Guide Overview

Welcome to the comprehensive guide for using Readur's document management system. This guide covers everything from basic operations to advanced features.

## Guide Structure

### Getting Started
- **[Installation](../getting-started/installation.md)** - Deploy Readur with Docker
- **[Quick Start](../getting-started/quickstart.md)** - 5-minute setup guide
- **[Configuration](../getting-started/configuration.md)** - Customize your deployment

### Core Features
- **[Document Management](../user-guide.md#document-management)** - Upload, organize, and manage documents
- **[OCR Processing](../user-guide.md#ocr-processing)** - Extract text from scanned documents
- **[Search & Discovery](../user-guide.md#search-features)** - Find information quickly
- **[Labels & Organization](../labels-and-organization.md)** - Categorize and structure content

### Advanced Features
- **[Sources & Sync](../sources-guide.md)** - Automated document import
- **[Advanced Search](../advanced-search.md)** - Complex queries and filters
- **[User Management](../user-management-guide.md)** - Roles and permissions
- **[API Integration](../api-reference.md)** - Programmatic access

### Administration
- **[Deployment](../deployment.md)** - Production setup and scaling
- **[Monitoring](../health-monitoring-guide.md)** - System health and metrics
- **[Backup & Recovery](../deployment.md#backup-strategy)** - Data protection
- **[Migration](../migration-guide.md)** - Upgrades and data migration

## Quick Navigation

### By User Type

#### Document Users
Start here if you need to:
- Upload and organize documents
- Search for specific content
- Export and share documents

**Key Guides:**
1. [User Guide](../user-guide.md)
2. [Search Features](../advanced-search.md)
3. [Labels Guide](../labels-and-organization.md)

#### System Administrators
Start here if you need to:
- Deploy and configure Readur
- Manage users and permissions
- Monitor system health
- Set up integrations

**Key Guides:**
1. [Installation](../getting-started/installation.md)
2. [Configuration](../configuration-reference.md)
3. [User Management](../user-management-guide.md)
4. [Deployment](../deployment.md)

#### Developers
Start here if you need to:
- Integrate with the API
- Customize Readur
- Contribute to development

**Key Guides:**
1. [API Reference](../api-reference.md)
2. [Development Setup](../dev/development.md)
3. [Architecture](../dev/architecture.md)

### By Task

#### Initial Setup
1. [Install Readur](../getting-started/installation.md)
2. [Configure OCR languages](../multi-language-ocr-guide.md)
3. [Set up authentication](../oidc-setup.md)
4. [Create users](../user-management-guide.md)

#### Document Processing
1. [Upload documents](../file-upload-guide.md)
2. [Configure OCR](../user-guide.md#ocr-processing)
3. [Monitor processing](../user-guide.md#ocr-status-indicators)
4. [Troubleshoot OCR](../dev/OCR_OPTIMIZATION_GUIDE.md)

#### Search & Organization
1. [Basic search](../user-guide.md#search-features)
2. [Advanced search syntax](../advanced-search.md)
3. [Create labels](../labels-and-organization.md)
4. [Save searches](../user-guide.md#smart-collections)

#### Integration & Automation
1. [Set up sources](../sources-guide.md)
2. [Configure watch folders](../WATCH_FOLDER.md)
3. [Use the API](../api-reference.md)
4. [Automate workflows](../api-reference.md#automation-examples)

## Feature Highlights

### Document Intelligence
- **OCR in 100+ Languages**: Process documents in virtually any language
- **Format Support**: PDF, images, Office documents, and text files
- **Batch Processing**: Handle thousands of documents efficiently
- **Quality Enhancement**: Automatic rotation, deskewing, and preprocessing

### Search Capabilities
- **Full-Text Search**: Search within document content
- **Boolean Logic**: Complex queries with AND, OR, NOT
- **Fuzzy Matching**: Handle OCR errors and typos
- **Filters**: By date, type, size, labels, and more

### Organization Tools
- **Flexible Labels**: Create custom categorization systems
- **Bulk Operations**: Apply changes to multiple documents
- **Smart Collections**: Saved searches that update automatically
- **Multiple Views**: List and grid layouts

### Integration Options
- **REST API**: Complete programmatic access
- **Source Sync**: WebDAV, S3, local folders
- **SSO/OIDC**: Enterprise authentication
- **Webhooks**: Event-driven automation

## Best Practices

### Document Organization
1. **Consistent Naming**: Use descriptive, standardized file names
2. **Label Strategy**: Create a hierarchical label structure
3. **Regular Cleanup**: Archive or remove outdated documents
4. **Folder Structure**: Organize source folders logically

### Performance Optimization
1. **OCR Settings**: Balance quality vs. speed for your needs
2. **Concurrent Jobs**: Match to available CPU cores
3. **Storage Backend**: Use S3 for large collections
4. **Search Indexing**: Schedule reindexing during off-hours

### Security
1. **Change Defaults**: Always change default passwords
2. **Enable HTTPS**: Use SSL/TLS in production
3. **Regular Backups**: Automate database backups
4. **Access Control**: Use roles and permissions appropriately

### Workflow Efficiency
1. **Bulk Upload**: Process similar documents together
2. **Automation**: Set up sources for automatic import
3. **Saved Searches**: Create shortcuts for common queries
4. **Keyboard Shortcuts**: Learn shortcuts for faster navigation

## Troubleshooting Resources

### Common Issues
- [OCR not starting](../user-guide.md#common-issues)
- [Search not finding documents](../advanced-search.md#troubleshooting)
- [Slow performance](../dev/OCR_OPTIMIZATION_GUIDE.md)
- [Upload failures](../file-upload-guide.md#troubleshooting)

### Getting Help
- **Documentation Search**: Use the search bar above
- **GitHub Issues**: [Report bugs](https://github.com/readur/readur/issues)
- **Community Forum**: [Ask questions](https://github.com/readur/readur/discussions)
- **System Logs**: Check logs for detailed error information

## Version Information

This documentation covers Readur version 2.5.4 and later. Key features in recent versions:

### Version 2.5.4
- S3 storage backend support
- Enhanced source synchronization
- Per-user watch directories
- Improved health monitoring

### Version 2.5.0
- OIDC/SSO authentication
- Advanced search operators
- Bulk operations
- Performance improvements

## Next Steps

### New Users
1. Start with the [Quick Start Guide](../getting-started/quickstart.md)
2. Read the [User Guide](../user-guide.md)
3. Learn about [Search Features](../advanced-search.md)

### Administrators
1. Review [Configuration Options](../configuration-reference.md)
2. Set up [Monitoring](../health-monitoring-guide.md)
3. Plan [Backup Strategy](../deployment.md#backup-strategy)

### Advanced Users
1. Explore [API Integration](../api-reference.md)
2. Configure [Sources](../sources-guide.md)
3. Optimize [OCR Performance](../dev/OCR_OPTIMIZATION_GUIDE.md)