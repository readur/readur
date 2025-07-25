# Labels and Organization Guide

Readur's labeling system provides powerful document organization and categorization capabilities. This guide covers creating, managing, and using labels to organize your document collection effectively.

## Table of Contents

- [Overview](#overview)
- [Label Types](#label-types)
- [Creating and Managing Labels](#creating-and-managing-labels)
- [Assigning Labels to Documents](#assigning-labels-to-documents)
- [Label-Based Search and Filtering](#label-based-search-and-filtering)
- [Label Organization Strategies](#label-organization-strategies)
- [Advanced Label Features](#advanced-label-features)
- [Best Practices](#best-practices)
- [API Integration](#api-integration)

## Overview

Labels in Readur provide a flexible tagging system that allows you to:

- **Categorize Documents**: Organize documents by type, project, department, or any custom criteria
- **Enhanced Search**: Filter search results by specific labels for precise document discovery
- **Visual Organization**: Color-coded labels provide instant visual categorization
- **Bulk Operations**: Apply or remove labels from multiple documents simultaneously
- **Project Management**: Track documents across projects, workflows, or time periods

### Key Features

- **Hierarchical Organization**: Create nested label structures for complex categorization
- **Color Coding**: Visual identification with customizable label colors
- **System Labels**: Automatic labels generated by Readur for administrative purposes
- **User Labels**: Custom labels created and managed by users
- **Smart Collections**: Save searches that automatically include documents with specific labels
- **Label Statistics**: Track document counts and usage analytics per label

## Label Types

### User Labels

**Custom labels** created and managed by users for personal or organizational categorization.

**Features:**
- **Full Control**: Create, edit, rename, and delete user-created labels
- **Color Customization**: Choose from a wide range of colors for visual organization
- **Flexible Naming**: Use any descriptive names that fit your workflow
- **Sharing**: Labels are visible to all users with access to labeled documents

**Common Use Cases:**
- Project names (e.g., "Project Alpha", "Q1 Budget")
- Document types (e.g., "Invoices", "Contracts", "Reports")
- Departments (e.g., "HR", "Engineering", "Marketing")
- Priority levels (e.g., "Urgent", "Review Needed", "Archive")
- Status indicators (e.g., "Draft", "Final", "Approved")

### System Labels

**Automatic labels** generated by Readur based on document properties and processing status.

**Examples:**
- **OCR Status**: "OCR Completed", "OCR Failed", "OCR Pending"
- **File Type**: "PDF", "Image", "Text Document"
- **Source Origin**: "WebDAV Upload", "Local Folder", "Manual Upload"
- **Processing Status**: "Recently Added", "High Confidence OCR", "Needs Review"
- **Size Categories**: "Large File", "Small File"
- **Date-based**: "This Week", "This Month", "This Year"

**Characteristics:**
- **Read-only**: Cannot be edited or deleted by users
- **Automatic Assignment**: Applied automatically based on document properties
- **System Managed**: Updated automatically when document properties change
- **Consistent Formatting**: Standardized naming and color scheme

## Creating and Managing Labels

### Creating New Labels

#### Via Label Management Page

1. **Navigate to Labels**: Go to Settings → Labels
2. **Click "Create Label"**
3. **Configure Label Properties**:
   ```
   Name: Project Documentation
   Color: Blue (#2196F3)
   Description: Documents related to current projects
   ```
4. **Save** to create the label

#### During Document Upload

1. **Upload Document(s)**: Use the upload interface
2. **Add Labels Field**: In the upload form
3. **Create New Label**: Type a new label name
4. **Assign Color**: Choose color for the new label
5. **Complete Upload**: Label is created and assigned automatically

#### Quick Label Creation

- **Search Interface**: Create labels while filtering search results
- **Document Details**: Add new labels directly from document pages
- **Bulk Operations**: Create labels during bulk document operations

### Editing Labels

#### Renaming Labels

1. **Access Label Management**: Settings → Labels
2. **Find Target Label**: Use search or browse the label list
3. **Click "Edit"** or double-click the label name
4. **Modify Name**: Change to new descriptive name
5. **Save Changes**: Updates all documents using this label

#### Changing Colors

1. **Edit Label**: Follow renaming steps above
2. **Select New Color**: Choose from color palette or enter hex code
3. **Preview Changes**: See how the color looks in different contexts
4. **Apply**: Color updates immediately across all interfaces

#### Merging Labels

1. **Identify Similar Labels**: Find labels with overlapping purposes
2. **Select Target Label**: Choose the label to keep
3. **Merge Operation**: Use "Merge with..." option
4. **Confirm Merge**: All documents transfer to target label
5. **Source Label Deletion**: Original label is removed after merge

### Deleting Labels

#### Individual Label Deletion

1. **Label Management Page**: Access via Settings → Labels
2. **Select Label**: Find the label to delete
3. **Delete Action**: Click delete button or menu option
4. **Confirm Deletion**: Confirm removal (this cannot be undone)
5. **Document Update**: Label is removed from all associated documents

#### Bulk Label Cleanup

- **Unused Labels**: Automatically identify and remove labels with no documents
- **Duplicate Labels**: Find and merge labels with similar names
- **Batch Deletion**: Select multiple labels for simultaneous removal

## Assigning Labels to Documents

### Single Document Labeling

#### Document Details Page

1. **Open Document**: Click on any document to view details
2. **Labels Section**: Find the labels area in document metadata
3. **Add Labels**: Click "+" or "Add Label" button
4. **Select or Create**: Choose existing labels or create new ones
5. **Apply Changes**: Labels are assigned immediately

#### Quick Label Assignment

- **Hover Actions**: Quick label buttons appear when hovering over documents
- **Right-Click Menu**: Context menu with common label operations
- **Keyboard Shortcuts**: Assign frequently used labels with key combinations

### Bulk Label Operations

#### Multi-Document Selection

1. **Document Browser**: Navigate to documents page
2. **Select Documents**: Use checkboxes to select multiple documents
3. **Bulk Actions**: Click "Actions" or "Labels" in the toolbar
4. **Apply Labels**: Choose labels to add or remove
5. **Execute**: Apply changes to all selected documents

#### Search-Based Labeling

1. **Search for Documents**: Use search to find specific document sets
2. **Select All Results**: Choose all documents matching criteria
3. **Bulk Label Assignment**: Apply labels to entire result set
4. **Confirmation**: Review and confirm bulk changes

### Label Assignment During Upload

#### Upload Interface Labeling

1. **File Selection**: Choose files to upload
2. **Label Assignment**: Add labels before starting upload
3. **Label Creation**: Create new labels during upload process
4. **Automatic Application**: Labels assigned to all uploaded files

#### Drag and Drop Labeling

- **Pre-configured Areas**: Drag files to labeled drop zones
- **Automatic Tagging**: Labels applied based on drop location
- **Batch Processing**: Assign labels to multiple files simultaneously

## Label-Based Search and Filtering

### Label Filters in Search

#### Basic Label Filtering

1. **Search Interface**: Access the main search page
2. **Label Filter Section**: Find label filters in the sidebar
3. **Select Labels**: Check boxes for desired labels
4. **Apply Filter**: Search results automatically update
5. **Multiple Labels**: Combine multiple labels with AND/OR logic

#### Advanced Label Queries

**Search Syntax Examples:**
```
label:urgent                    # Documents with "urgent" label
label:"project alpha"           # Documents with multi-word label
label:urgent AND label:review   # Documents with both labels
label:draft OR label:final      # Documents with either label
-label:archive                  # Exclude archived documents
```

### Smart Collections

#### Creating Smart Collections

1. **Build Search Query**: Create search with label filters
2. **Save Search**: Use "Save Search" option
3. **Name Collection**: Give descriptive name (e.g., "Active Projects")
4. **Automatic Updates**: Collection updates as documents are labeled
5. **Quick Access**: Access collections from sidebar or dashboard

#### Collection Examples

**Project-Based Collections:**
- "Q1 Budget Documents": `label:"Q1 budget" OR label:"financial planning"`
- "Marketing Materials": `label:marketing AND (label:final OR label:approved)`
- "Pending Review": `label:"needs review" AND -label:completed`

**Status-Based Collections:**
- "Recent Uploads": `label:"this month" AND -label:processed`
- "High Priority": `label:urgent OR label:critical`
- "Archive Ready": `label:completed AND label:final`

### Label-Based Dashboard Views

#### Custom Dashboard Widgets

- **Label Statistics**: Show document counts per label
- **Recent Activity**: Display recently labeled documents
- **Label Trends**: Track labeling patterns over time
- **Quick Access**: Direct links to frequently used label filters

## Label Organization Strategies

### Hierarchical Labeling

#### Category-Based Organization

**Structure Example:**
```
Projects/
├── Project Alpha/
│   ├── Requirements
│   ├── Design
│   └── Implementation
├── Project Beta/
│   ├── Research
│   ├── Proposals
│   └── Contracts
└── Infrastructure/
    ├── Servers
    ├── Network
    └── Security
```

#### Implementation Approach

1. **Top-Level Categories**: Create broad organizational labels
2. **Subcategories**: Use descriptive naming for specific areas
3. **Consistent Naming**: Establish naming conventions across categories
4. **Cross-References**: Documents can belong to multiple hierarchies

### Functional Organization

#### Document Lifecycle Labels

**Workflow Stages:**
- **Creation**: "Draft", "In Progress", "Under Review"
- **Approval**: "Pending Approval", "Approved", "Rejected"
- **Distribution**: "Published", "Distributed", "Archived"
- **Maintenance**: "Current", "Outdated", "Superseded"

#### Department-Based Labeling

**Organizational Structure:**
- **Human Resources**: "HR Policy", "Employee Records", "Benefits"
- **Finance**: "Invoices", "Budget", "Audit", "Tax Documents"
- **Legal**: "Contracts", "Compliance", "IP Documents"
- **Operations**: "Procedures", "Manuals", "Incident Reports"

### Time-Based Organization

#### Date-Driven Labels

- **Fiscal Periods**: "Q1 2024", "FY2024", "H1 2024"
- **Project Phases**: "Phase 1", "Phase 2", "Final Phase"
- **Event-Based**: "Pre-Launch", "Launch", "Post-Launch"
- **Seasonal**: "Annual Review", "Budget Season", "Audit Period"

## Advanced Label Features

### Label Analytics

#### Usage Statistics

**Metrics Available:**
- **Document Count**: Number of documents per label
- **Recent Activity**: Labels used in recent uploads or assignments
- **Growth Trends**: How label usage changes over time
- **Popular Labels**: Most frequently used labels
- **Unused Labels**: Labels with no current document assignments

#### Label Performance

- **Search Frequency**: How often labels are used in searches
- **Click-Through Rates**: User engagement with labeled content
- **Organization Effectiveness**: How labels improve document discovery

### Label Automation

#### Auto-Labeling Rules

**OCR-Based Labeling:**
- **Content Detection**: Automatically label documents based on detected text
- **Template Recognition**: Recognize document types and apply appropriate labels
- **Entity Extraction**: Label documents based on detected entities (names, dates, amounts)

**Source-Based Labeling:**
- **Upload Location**: Apply labels based on upload source or folder
- **File Type**: Automatic labels based on file format and structure
- **Metadata**: Labels derived from file properties and EXIF data

#### Workflow Integration

- **Process Triggers**: Apply labels based on workflow stage completion
- **Approval Status**: Automatic labeling based on approval workflows
- **Time-Based Rules**: Apply labels based on document age or schedule

### Label Import/Export

#### Bulk Label Operations

**Import Scenarios:**
- **Migration**: Import existing label structures from other systems
- **Template Application**: Apply predefined label sets to document collections
- **Organizational Standards**: Implement company-wide labeling standards

**Export Capabilities:**
- **Backup**: Export label definitions for backup purposes
- **Reporting**: Generate reports of label usage and document organization
- **Integration**: Share label structures with other systems

## Best Practices

### Label Design

#### Naming Conventions

1. **Descriptive Names**: Use clear, self-explanatory label names
2. **Consistent Format**: Establish and follow naming patterns
3. **Avoid Ambiguity**: Choose names that won't be confused with similar concepts
4. **Length Consideration**: Keep names concise but informative
5. **Special Characters**: Avoid special characters that may cause issues

**Good Examples:**
- "Q1-2024-Budget" ✅
- "Legal-Contract-Template" ✅
- "Marketing-Campaign-Assets" ✅

**Poor Examples:**
- "Stuff" ❌ (too vague)
- "Q1 Budget Documents for 2024 Financial Planning" ❌ (too long)
- "Legal/Contract#Template@2024" ❌ (special characters)

#### Color Strategy

1. **Consistent Color Families**: Use similar colors for related label categories
2. **High Contrast**: Ensure labels are readable against various backgrounds
3. **Color Meaning**: Establish color conventions (e.g., red for urgent, green for completed)
4. **Accessibility**: Consider color-blind users when choosing colors
5. **Limited Palette**: Don't use too many different colors

### Organization Strategy

#### Start Simple

1. **Basic Categories**: Begin with broad, obvious categories
2. **Organic Growth**: Add labels as needs become apparent
3. **User Feedback**: Incorporate user suggestions for new labels
4. **Regular Review**: Periodically assess and refine label structure

#### Maintain Consistency

1. **Documentation**: Document labeling standards and conventions
2. **Training**: Educate users on proper labeling practices
3. **Regular Cleanup**: Remove unused or redundant labels
4. **Standardization**: Ensure consistent application across teams

### Performance Optimization

#### Label Management

1. **Avoid Over-Labeling**: Don't create too many similar labels
2. **Regular Cleanup**: Remove unused labels to reduce clutter
3. **Search Optimization**: Focus on labels that improve searchability
4. **User Training**: Educate users on effective labeling practices

#### System Performance

- **Index Optimization**: Labels are indexed for fast search performance
- **Bulk Operations**: Use bulk assignment for better efficiency
- **Caching**: Frequently used labels are cached for quick access

## API Integration

### Label Management API

#### Creating Labels

```bash
POST /api/labels
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "name": "Project Documentation",
  "color": "#2196F3"
}
```

#### Listing Labels

```bash
GET /api/labels
Authorization: Bearer <jwt_token>
```

Response:
```json
{
  "labels": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "Project Documentation",
      "color": "#2196F3",
      "document_count": 42,
      "created_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

#### Assigning Labels to Documents

```bash
PATCH /api/documents/{document_id}
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "labels": ["Project Documentation", "Q1 2024", "High Priority"]
}
```

### Search Integration

#### Label-Based Search

```bash
GET /api/search?query=invoice&labels=urgent,review
Authorization: Bearer <jwt_token>
```

#### Advanced Label Queries

```bash
POST /api/search/advanced
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "query": "budget",
  "filters": {
    "labels": ["Q1 2024", "Finance"],
    "label_logic": "AND"
  }
}
```

## Next Steps

- Configure [advanced search](advanced-search.md) with label-based filtering
- Set up [sources](sources-guide.md) with automatic labeling rules
- Explore [user management](user-management-guide.md) for collaborative labeling
- Review [API reference](api-reference.md) for programmatic label management
- Check [best practices](user-guide.md#tips-for-best-results) for document organization