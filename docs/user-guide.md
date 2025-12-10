# User Guide

This guide walks you through everything you need to know to effectively use Readur for managing, searching, and organizing your documents. Whether you're new to document management systems or coming from another platform, you'll find practical guidance for making the most of Readur's features.

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

When you first access Readur, navigate to your installation URL (typically `http://localhost:8000` for local installations) and log in with the admin credentials. The username is `admin` and the password is auto-generated on first startup - check your container logs (`docker compose logs readur`) and look for "READUR ADMIN USER CREATED" to find your password. Save this password immediately as it won't be shown again.

Take a moment to configure your user preferences through the settings menu. If you work with documents in languages other than English, set your preferred OCR language now - this will improve text extraction accuracy for your documents. You can also adjust search settings and display preferences to match how you like to work.

To get familiar with Readur, try uploading your first document. You can either drag and drop a file onto the upload area or use the upload button to select files. After uploading, wait for the OCR processing to complete (you'll see a progress indicator), then try searching for text you know is in the document. This will give you a feel for how Readur extracts and indexes content.

## Supported File Types

| Type | Extensions | OCR Support | Notes |
|------|-----------|-------------|-------|
| **PDF** | `.pdf` | ✅ | Text extraction + OCR for scanned pages |
| **Images** | `.png`, `.jpg`, `.jpeg`, `.tiff`, `.bmp`, `.gif` | ✅ | Full OCR text extraction |
| **Text** | `.txt`, `.rtf` | ❌ | Direct text indexing |
| **Office** | `.doc`, `.docx` | ⚠️ | Limited support |

## Using the Interface

### Dashboard Overview

The dashboard gives you a quick view of your document system's health and activity. You'll see statistics about how many documents you have, how much storage you're using, and the status of any OCR processing currently running. The recent activity timeline shows what's been happening in your system, which is helpful for tracking when documents were uploaded or processed.

From the dashboard, you can quickly upload new documents, use the search bar to find content, and access your most recently viewed documents. Any system notifications or alerts will also appear here, keeping you informed about important events or issues that need attention.

### Document Management

#### Choosing Your View

Readur offers two ways to view your documents, and you can switch between them using the view selector in the top toolbar. List view shows detailed information in a table format with columns for file name, upload date, size, type, and processing status. This view is ideal when you need to see metadata at a glance or work with large numbers of documents.

Grid view displays visual thumbnails of your documents, making it easier to identify files by their content. This view works particularly well when you're looking for a specific document and can recognize it visually, or when you want to browse through documents more casually.

#### Organizing Your Documents

You can sort your documents by upload date (newest or oldest first), file name alphabetically, file size, document type, or OCR processing status. This flexibility helps you find what you're looking for whether you remember when you uploaded something, what it was called, or what type of file it was.

Filtering helps narrow down large document collections. Filter by file type if you're only looking for PDFs or images, by OCR status to see which documents still need processing, by date range to find documents from a specific time period, by the labels you've applied, or by source if you want to see only documents from a particular watch folder or sync source.

#### Working with Multiple Documents

When you need to perform actions on several documents at once, use the checkboxes to select them. Once selected, you can delete multiple documents, add or remove labels in bulk, export a list of the selected documents, or trigger OCR reprocessing if needed. This saves significant time when organizing large numbers of documents.

### Advanced Search

Readur's search capabilities are designed to help you find documents quickly, even when you can't remember exact details.

#### How Search Works

The full-text search looks through all the text extracted from your documents, not just file names. It automatically handles word variations (stemming) and can find matches even when there are minor typos or OCR errors (fuzzy matching). When you search for "running," it will also find documents containing "run" or "runs."

For exact phrase searches, put your terms in quotes like `"quarterly report"` to find that specific phrase. To exclude certain terms, use a minus sign: `invoice -draft` will find invoices but exclude any documents containing the word "draft."

#### Refining Your Search

Use the search filters to narrow down results when you have too many matches. Set a date range if you remember roughly when the document was created or uploaded. Filter by file type when you know you're looking for a PDF versus an image. You can also filter by the labels you've applied to documents or by which source they came from.

The search syntax gives you powerful control when you need it. Search for `tag:important invoice` to find invoices you've labeled as important, or `type:pdf contract` to find only PDF files containing the word "contract." Combine multiple techniques: `"project status" tag:urgent 2024` finds documents with the exact phrase "project status" that are labeled urgent and contain "2024."

### Sources and Synchronization

Readur can automatically import documents from external storage systems, saving you from manually uploading files. This feature works with WebDAV servers (like Nextcloud or ownCloud), local folders and network mounts, and S3-compatible storage services. The process is non-destructive, meaning your original files stay exactly where they are while Readur maintains its own searchable copies.

Sources sync on schedules you configure, and Readur monitors the health of these connections to alert you if something goes wrong. The system is smart about avoiding duplicate work - it only processes new or changed files during each sync, and it integrates seamlessly with OCR processing.

#### Source Types You Can Use

WebDAV sources connect to cloud storage services like Nextcloud, ownCloud, or any generic WebDAV server. This is useful when your organization stores documents in a shared cloud system. Local folder sources monitor directories on your server's filesystem or mounted network drives, perfect for traditional file server setups. S3 sources work with Amazon S3 or compatible services like MinIO and DigitalOcean Spaces, giving you cloud-scale document storage.

#### Setting Up a Source

To add a source, go to Settings → Sources and click "Add Source." Choose your source type and provide the connection details and credentials. Test the connection to make sure Readur can access your storage, then configure which folders to monitor and how often to sync. You can set different sync schedules for different sources based on how frequently they receive new documents.

> 📖 **For comprehensive source configuration**, see the [Sources Guide](sources-guide.md)

## Document Upload

### Getting Documents into Readur

The most straightforward way to add documents is through manual upload. Click the upload button or simply drag files from your file manager directly onto the upload area. You can select single files or multiple files at once, and you have the option to add labels during the upload process, which helps with organization right from the start.

Drag and drop works from anywhere on the document list page, making it convenient to add files while you're browsing your existing documents. You can drop multiple files simultaneously, and Readur will queue them all for processing.

Keep in mind the system's upload limits, which are typically configured for a maximum file size of 50MB by default (though this can be adjusted by your administrator). You can upload up to 100 files in a single batch, and all the file types listed in the supported formats table are accepted.

## OCR Processing

### How OCR Works in Readur

After you upload a document, OCR processing starts automatically in the background. You don't need to do anything - Readur handles the entire process. The system uses a priority queue that processes smaller files first, so quick documents finish in seconds while larger batches work through the queue efficiently.

### Configuring OCR Settings

You can adjust OCR settings to improve accuracy for your specific use case. Choose from over 100 languages - Readur will auto-detect in many cases, but setting your preferred language explicitly helps with accuracy. Enable image preprocessing to enhance scanned documents with poor quality, and turn on auto-rotation to correct documents that were scanned upside down or sideways. You can also balance the quality setting between speed and accuracy based on your priorities.

### Understanding OCR Status

Each document shows its OCR status with clear indicators. A green indicator means text extraction is complete and the document is fully searchable. Yellow indicates processing is currently in progress. Red means there was an error during processing (often due to very poor image quality or unsupported content), and white means the document is waiting in the queue to be processed. You can always trigger reprocessing if needed from the document's detail page.

## Search Features

Readur's search system gives you multiple ways to find documents, from simple keyword searches to complex queries with boolean logic.

### Different Ways to Search

Simple search works for most situations - just type what you're looking for and Readur automatically handles word variations and minor typos. When you need to find an exact phrase, put it in quotes like `"quarterly report"`. For documents that might have OCR errors or typos, use fuzzy search by adding a tilde: `invoice~` will find "invoice" even if it was read as "invioce." When you need precise control, boolean search lets you combine terms with AND, OR, and NOT operators.

### Using the Search Interface

The quick search bar in the header gives you instant access from any page. As you type, it shows results immediately with text snippets from matching documents. For more complex searches, use the advanced search page where you can access all filtering options, save frequently used searches, and export your results. The advanced page also tracks your search history and provides analytics about what you search for most often.

### Filtering Your Results

When your search returns too many results, filters help narrow things down. Filter by file type when you know you're looking for a PDF versus an image. Set date ranges to find documents from specific time periods. Use label filters to search within documents you've categorized. Filter by source if you want results from only specific sync sources. You can also filter by file size ranges or OCR processing status.

### Search Techniques That Work

For exact phrases, always use quotes: `"project status"` finds that specific phrase rather than documents containing both words separately. Combine text searches with filters for more precise results - search for "budget" and filter by file type "PDF" to find budget PDFs specifically. Use wildcards when you're not sure of exact word endings: `proj*` matches project, projects, and projection. Search specific fields with syntax like `filename:report` or `label:urgent`. For complex needs, use boolean logic: `(budget OR financial) AND 2024` finds documents containing either "budget" or "financial" that also contain "2024."

> 🔍 **For detailed search techniques**, see the [Advanced Search Guide](advanced-search.md)

## Labels and Organization

Labels are your primary tool for organizing documents in Readur. Unlike traditional folder systems where a file can only be in one place, labels let you apply multiple categories to the same document. A single invoice could be labeled as "Finance," "2024," "Quarterly," and "Approved" simultaneously, making it findable through any of those organizational schemes.

### Understanding Label Types

Readur uses two main types of labels. User labels are the ones you create and manage - these reflect your personal or organizational categorization needs. System labels are automatically generated based on document properties like file type, OCR status, or upload source. You can use both types in searches and filters, but only user labels can be customized.

You can assign colors to your labels for visual organization. This becomes particularly helpful when you're scanning through document lists - important documents with red labels stand out immediately, while routine items might use neutral colors. The system also supports hierarchical labels where you can create categories and subcategories for complex organizational schemes.

### Creating and Managing Your Labels

There are several ways to create labels depending on when you think of them. During upload, you can create and assign labels as you add documents, which helps establish organization from the start. You can also create labels through the Settings → Labels page, which is useful for planning your organizational structure before you start labeling documents.

When viewing individual documents, you can add labels directly from the document detail page. This approach works well when you discover new categorization needs while reviewing content. For efficiency with multiple documents, use bulk operations to create and assign labels to entire sets of documents at once.

Managing labels is straightforward - you can rename labels and all associated documents update automatically. When you discover you have similar or duplicate labels, you can merge them to consolidate your organization scheme. Color management helps maintain visual consistency across your label system.

### Developing Your Organization Strategy

Most successful Readur users develop a consistent labeling strategy that reflects how they actually think about and use their documents. Category-based organization works well for many use cases - you might create labels for different projects, departments, document types, or processing status. For example, a document could have labels like "Project Alpha," "Finance," "Contract," and "Final" all at once.

Time-based organization adds another valuable dimension. Consider creating labels for fiscal periods, project phases, or significant events. This approach makes it easy to find all documents related to a specific time frame, like "Q1 2024" or "Pre-Launch." You can combine time-based labels with category labels for powerful organization schemes.

### Using Smart Collections

Smart Collections are saved searches that automatically update as you add new documents. Instead of manually maintaining lists of documents, you create search criteria and Readur continuously finds documents that match. For instance, you could create a "Current Projects" collection that automatically includes any document labeled with active project names, or a "Pending Review" collection that shows all documents with review-related labels.

These collections become particularly powerful when combined with boolean search logic. A "High Priority Items" collection might search for documents with labels like "urgent" OR "critical" OR "immediate," automatically gathering important documents regardless of which specific urgency label was used.

> 🏷️ **For comprehensive labeling strategies**, see the [Labels and Organization Guide](labels-and-organization.md)

## User Management

Readur offers flexible user management that works for both small teams and large organizations. You can use simple username and password authentication for straightforward setups, or integrate with enterprise identity providers for seamless single sign-on in corporate environments.

### Authentication Options

For most small to medium installations, local authentication provides everything you need. Users create accounts with usernames and passwords, which Readur stores securely using industry-standard bcrypt hashing. You can enable self-registration to let users create their own accounts, or keep it admin-only for tighter control.

Enterprise environments often benefit from OIDC/SSO integration, which lets users authenticate with their existing corporate credentials. Readur supports major identity providers including Microsoft Azure AD, Google Workspace, Okta, Auth0, and Keycloak. When someone logs in for the first time through SSO, Readur automatically creates their account, streamlining the onboarding process.

### Understanding User Roles

Readur uses a straightforward two-tier permission system. Regular users can upload and manage documents, search through the collection, configure their personal settings, create and manage labels, and set up their own document sources. This covers everything most users need for daily document management.

Administrators have additional capabilities for system management. They can create, modify, and delete user accounts, configure global system settings, assign user roles, and monitor system health and performance. The admin role is designed for technical staff who need to maintain the Readur installation.

### Managing Users as an Administrator

The user management interface in Settings → Users gives administrators full control over user accounts. You can create new users and assign them roles immediately, modify existing user information including passwords and roles, and get an overview of all users with their creation dates and current roles.

The system handles both local and OIDC users seamlessly - you can have some users authenticate locally while others use corporate SSO, all managed through the same interface. For efficiency, bulk operations let you perform actions on multiple users simultaneously.

### Supporting Mixed Authentication

Many organizations start with local authentication and later add SSO integration, or use both simultaneously. Readur handles this gracefully - you might maintain local admin accounts for system management while regular users authenticate through your corporate identity provider. Role assignment works the same way regardless of how someone authenticates, giving you consistent permission management across all authentication methods.

> 👥 **For detailed user administration**, see the [User Management Guide](user-management-guide.md)
> 🔐 **For OIDC configuration**, see the [OIDC Setup Guide](oidc-setup.md)

## User Settings

### Customizing Your Experience

Readur provides several settings to tailor the interface to your preferences and workflow. Your display settings control how you see documents by default - choose between list view for detailed information or grid view for visual browsing. Set your interface language and time zone to ensure timestamps and dates appear correctly for your location.

Notification settings let you control how Readur alerts you about important events like completed OCR processing, sync status updates, or system messages. You can choose between email notifications, in-app alerts, or both, depending on how you prefer to stay informed.

### Optimizing OCR for Your Documents

Your OCR preferences significantly impact how well Readur processes your documents. Setting your default OCR language to match most of your content improves accuracy, though you can still specify different languages for individual uploads when needed. The processing priority setting affects how quickly your documents move through the OCR queue.

Image preprocessing options help with documents that aren't perfectly scanned. Enable this feature if you frequently work with photos taken by phone cameras or scans with poor lighting. Batch size limits control how many documents Readur processes simultaneously, which you might adjust based on your system's performance and your typical upload patterns.

### Configuring Search Behavior

Search settings affect how results appear and behave. Adjust the number of results per page based on your screen size and browsing preferences. The default sort order determines how search results are organized - by relevance, date, or other criteria that match how you typically look for documents.

Snippet length controls how much text appears in search previews, while the fuzzy search threshold determines how tolerant the system is of typos and OCR errors. Higher thresholds find more matches but might include less relevant results, while lower thresholds are more precise but might miss documents with text recognition issues.

## Tips for Best Results

### Getting the Most from OCR

The quality of your source documents dramatically affects OCR accuracy. When scanning documents, aim for 300 DPI or higher resolution - this provides enough detail for the OCR engine to distinguish individual characters clearly. Keep your scans straight and clean - skewed or dirty documents with spots and stains confuse the text recognition process.

For documents you photograph with a phone or camera, ensure even lighting across the entire page. Shadows and glare create areas where text becomes difficult to read. Black text on white backgrounds produces the most reliable results, though Readur can handle colored text and backgrounds with varying degrees of success.

### Organizing Files Effectively

Develop consistent naming conventions for your files before uploading them to Readur. Descriptive, consistent file names make documents easier to identify even before OCR processing completes. Instead of letting documents accumulate, upload them regularly to keep your system current and prevent overwhelming the OCR queue.

Apply labels immediately after uploading documents while the content is fresh in your mind. This practice ensures consistent organization and makes documents findable through multiple approaches. If you use watch folders or sync sources, organize them logically to reflect how you naturally categorize documents.

### Maximizing Search Effectiveness

Combine text searches with filters to get more precise results, especially when working with large document collections. Save frequently used search queries as Smart Collections to avoid retyping complex searches. Invest time in learning the search syntax - advanced operators like boolean logic and field-specific searches become invaluable as your document collection grows.

Keep an eye on processing status to ensure all documents have been indexed. Unprocessed documents won't appear in search results, which can be confusing if you're expecting to find recently uploaded content.

### Optimizing System Performance

Upload similar documents together when possible - this helps the OCR system optimize its processing approach for consistent document types. For large batches of documents, consider uploading during off-peak hours to avoid impacting other users or system responsiveness.

Monitor the OCR queue status regularly, especially after large uploads. If you notice persistent backlogs, you might need to adjust your upload timing or discuss resource allocation with your system administrator. Periodically review your document collection and remove outdated files to keep storage usage reasonable and search results relevant.

## Troubleshooting

### When OCR Isn't Working

If documents aren't being processed for OCR, start by checking that your files are within the configured size limits and in supported formats. The system only processes file types it recognizes, so unusual extensions or corrupted files might be ignored. Verify that the OCR service is actually running by checking the system status page or asking your administrator.

Sometimes OCR appears stuck when it's actually working through a large backlog. Check the processing queue to see how many documents are waiting ahead of yours. Very large files or high-resolution images take significantly longer to process than text documents.

### When Search Isn't Finding Documents

The most common search problem is looking for content in documents that haven't finished OCR processing yet. Check that the documents you're searching have green status indicators showing successful text extraction. If OCR failed (red indicator), the document text won't be searchable.

Review your search syntax if you're using advanced operators - misplaced quotes or boolean operators can prevent matches. When in doubt, try simpler, broader search terms first, then add complexity once you're getting basic results.

### When Performance Is Slow

System slowness usually relates to resource allocation and concurrent processing. If searches are slow, your document collection might have grown beyond the database's current indexing capacity. If uploads and OCR processing are slow, check whether too many concurrent jobs are running for your system's CPU and memory resources.

Large file uploads can temporarily impact performance for other users. Consider spreading large upload sessions across time or scheduling them during low-usage periods. Your system administrator can adjust memory limits and concurrent processing settings to better match your hardware capabilities.

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