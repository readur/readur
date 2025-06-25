# User Guide

A comprehensive guide to using Readur's features for document management, OCR processing, and search.

## Table of Contents

- [Getting Started](#getting-started)
- [Supported File Types](#supported-file-types)
- [Using the Interface](#using-the-interface)
  - [Dashboard](#dashboard)
  - [Document Management](#document-management)
  - [Advanced Search](#advanced-search)
  - [Folder Watching](#folder-watching)
- [Document Upload](#document-upload)
- [OCR Processing](#ocr-processing)
- [Search Features](#search-features)
- [Tags and Organization](#tags-and-organization)
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

### Folder Watching

The folder watching feature automatically imports documents:

1. **Non-destructive**: Source files remain untouched
2. **Automatic Processing**: New files are detected and processed
3. **Configurable Intervals**: Adjust scan frequency
4. **Multiple Sources**: Watch local folders, network drives, cloud storage

#### Setting Up Watch Folders
1. Go to Settings → Sources
2. Add a new source with type "Local Folder"
3. Configure the path and scan interval
4. Enable/disable the source as needed

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

### Quick Search
- Available in the header on all pages
- Instant results as you type
- Shows top 5 matches with snippets

### Advanced Search Page
- Full search interface with all filters
- Export search results
- Save frequently used searches
- Search history

### Search Tips
1. Use quotes for exact phrases
2. Combine filters for precise results
3. Use wildcards: `inv*` matches invoice, inventory
4. Search in specific fields: `filename:report`

## Tags and Organization

### Creating Tags
1. Select document(s)
2. Click "Add Tag"
3. Enter tag name or select existing
4. Tags are color-coded for easy identification

### Tag Management
- Rename tags globally
- Merge similar tags
- Delete unused tags
- Set tag colors

### Smart Collections
Create saved searches based on:
- Tag combinations
- Date ranges
- File types
- Custom criteria

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

- Explore the [API Reference](api-reference.md) for automation
- Learn about [advanced configuration](configuration.md)
- Set up [automated workflows](WATCH_FOLDER.md)
- Optimize [OCR performance](dev/OCR_OPTIMIZATION_GUIDE.md)