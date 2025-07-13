# Advanced Search Guide

Readur provides powerful search capabilities that go far beyond simple text matching. This comprehensive guide covers all search modes, advanced filtering, query syntax, and optimization techniques.

## Table of Contents

- [Overview](#overview)
- [Search Modes](#search-modes)
- [Query Syntax](#query-syntax)
- [Advanced Filtering](#advanced-filtering)
- [Search Interface](#search-interface)
- [Search Optimization](#search-optimization)
- [Saved Searches](#saved-searches)
- [Search Analytics](#search-analytics)
- [API Search](#api-search)
- [Troubleshooting](#troubleshooting)

## Overview

Readur's search system is built on PostgreSQL's full-text search capabilities with additional enhancements for document-specific requirements.

### Search Capabilities

- **Full-Text Search**: Search within document content and OCR-extracted text
- **Multiple Search Modes**: Simple, phrase, fuzzy, and boolean search options
- **Advanced Filtering**: Filter by file type, date, size, labels, and source
- **Real-Time Suggestions**: Auto-complete and query suggestions as you type
- **Faceted Search**: Browse documents by categories and properties
- **Cross-Language Support**: Search in multiple languages with OCR text
- **Relevance Ranking**: Intelligent scoring and result ordering

### Search Sources

Readur searches across multiple content sources:

1. **Document Content**: Original text from text files and PDFs
2. **OCR Text**: Extracted text from images and scanned documents  
3. **Metadata**: File names, descriptions, and document properties
4. **Labels**: User-created and system-generated tags
5. **Source Information**: Upload source and file paths

## Search Modes

### Simple Search (Smart Search)

**Best for**: General purpose searching and quick document discovery

**How it works**:
- Automatically applies stemming and fuzzy matching
- Searches across all text content and metadata
- Provides intelligent relevance scoring
- Handles common typos and variations

**Example**:
```
invoice 2024
```
Finds: "Invoice Q1 2024", "invoicing for 2024", "2024 invoice data"

**Features**:
- **Auto-stemming**: "running" matches "run", "runs", "runner"
- **Fuzzy tolerance**: "recieve" matches "receive"
- **Partial matching**: "doc" matches "document", "documentation"
- **Relevance ranking**: More relevant matches appear first

### Phrase Search (Exact Match)

**Best for**: Finding exact phrases or specific terminology

**How it works**:
- Searches for the exact sequence of words
- Case-insensitive but order-sensitive
- Useful for finding specific quotes, names, or technical terms

**Syntax**: Use quotes around the phrase
```
"quarterly financial report"
"John Smith"
"error code 404"
```

**Features**:
- **Exact word order**: Only matches the precise sequence
- **Case insensitive**: "John Smith" matches "john smith"
- **Punctuation ignored**: "error-code" matches "error code"

### Fuzzy Search (Approximate Matching)

**Best for**: Handling typos, OCR errors, and spelling variations

**How it works**:
- Uses trigram similarity to find approximate matches
- Configurable similarity threshold (default: 0.8)
- Particularly useful for OCR-processed documents with errors

**Syntax**: Use the `~` operator
```
invoice~     # Finds "invoice", "invoce", "invoise"
contract~    # Finds "contract", "contarct", "conract"
```

**Configuration**:
- **Threshold adjustment**: Configure sensitivity via user settings
- **Language-specific**: Different languages may need different thresholds
- **OCR optimization**: Higher tolerance for OCR-processed documents

### Boolean Search (Logical Operators)

**Best for**: Complex queries with multiple conditions and precise control

**Operators**:
- **AND**: Both terms must be present
- **OR**: Either term can be present  
- **NOT**: Exclude documents with the term
- **Parentheses**: Group conditions

**Examples**:
```
budget AND 2024                    # Both "budget" and "2024"
invoice OR receipt                  # Either "invoice" or "receipt"
contract NOT draft                  # "contract" but not "draft"
(budget OR financial) AND 2024      # Complex grouping
marketing AND (campaign OR strategy) # Marketing documents about campaigns or strategy
```

**Advanced Boolean Examples**:
```
# Find completed project documents
project AND (final OR completed OR approved) NOT draft

# Financial documents excluding personal items
(invoice OR receipt OR budget) NOT personal

# Recent important documents
(urgent OR priority OR critical) AND label:"this month"
```

## Query Syntax

### Field-Specific Search

Search within specific document fields for precise targeting.

#### Available Fields

| Field | Description | Example |
|-------|-------------|---------|
| `filename:` | Search in file names | `filename:invoice` |
| `content:` | Search in document text | `content:"project status"` |
| `label:` | Search by labels | `label:urgent` |
| `type:` | Search by file type | `type:pdf` |
| `source:` | Search by upload source | `source:webdav` |
| `size:` | Search by file size | `size:>10MB` |
| `date:` | Search by date | `date:2024-01-01` |

#### Field Search Examples

```
filename:contract AND date:2024        # Contracts from 2024
label:"high priority" OR label:urgent  # Priority documents
type:pdf AND content:budget            # PDF files containing "budget"
source:webdav AND label:approved       # Approved docs from WebDAV
```

### Range Queries

#### Date Ranges
```
date:2024-01-01..2024-03-31    # Q1 2024 documents
date:>2024-01-01               # After January 1, 2024
date:<2024-12-31               # Before December 31, 2024
```

#### Size Ranges
```
size:1MB..10MB                 # Between 1MB and 10MB
size:>50MB                     # Larger than 50MB
size:<1KB                      # Smaller than 1KB
```

### Wildcard Search

Use wildcards for partial matching:

```
proj*           # Matches "project", "projects", "projection"
*report         # Matches "annual report", "status report"
doc?ment        # Matches "document", "documents" (? = single character)
```

### Exclusion Operators

Exclude unwanted results:

```
invoice -draft                 # Invoices but not drafts
budget NOT personal           # Budget documents excluding personal
-label:archive proposal       # Proposals not in archive
```

## Advanced Filtering

### File Type Filters

Filter by specific file formats:

**Common File Types**:
- **Documents**: PDF, DOC, DOCX, TXT, RTF
- **Images**: PNG, JPG, JPEG, TIFF, BMP, GIF
- **Spreadsheets**: XLS, XLSX, CSV
- **Presentations**: PPT, PPTX

**Filter Interface**:
1. **Checkbox Filters**: Select multiple file types
2. **MIME Type Groups**: Filter by general categories
3. **Custom Extensions**: Add specific file extensions

**Search Syntax**:
```
type:pdf                       # Only PDF files
type:(pdf OR doc)              # PDF or Word documents
-type:image                    # Exclude all images
```

### Date and Time Filters

**Predefined Ranges**:
- Today, Yesterday, This Week, Last Week
- This Month, Last Month, This Quarter, Last Quarter
- This Year, Last Year

**Custom Date Ranges**:
- **Start Date**: Documents uploaded after specific date
- **End Date**: Documents uploaded before specific date
- **Date Range**: Documents within specific period

**Advanced Date Syntax**:
```
created:today                  # Documents uploaded today
modified:>2024-01-01          # Modified after January 1st
accessed:last-week            # Accessed in the last week
```

### Size Filters

**Size Categories**:
- **Small**: < 1MB
- **Medium**: 1MB - 10MB  
- **Large**: 10MB - 50MB
- **Very Large**: > 50MB

**Custom Size Ranges**:
```
size:>10MB                     # Larger than 10MB
size:1MB..5MB                  # Between 1MB and 5MB
size:<100KB                    # Smaller than 100KB
```

### Label Filters

**Label Selection**:
- **Multiple Labels**: Select multiple labels with AND/OR logic
- **Label Hierarchy**: Navigate nested label structures
- **Label Suggestions**: Auto-complete based on existing labels

**Label Search Syntax**:
```
label:project                  # Documents with "project" label
label:"high priority"          # Multi-word labels in quotes
label:(urgent OR critical)     # Documents with either label
-label:archive                 # Exclude archived documents
```

### Source Filters

Filter by document source or origin:

**Source Types**:
- **Manual Upload**: Documents uploaded directly
- **WebDAV Sync**: Documents from WebDAV sources
- **Local Folder**: Documents from watched folders
- **S3 Sync**: Documents from S3 buckets

**Source-Specific Filters**:
```
source:webdav                  # WebDAV synchronized documents
source:manual                  # Manually uploaded documents
source:"My Nextcloud"          # Specific named source
```

### OCR Status Filters

Filter by OCR processing status:

**Status Options**:
- **Completed**: OCR successfully completed
- **Pending**: Waiting for OCR processing
- **Failed**: OCR processing failed
- **Not Applicable**: Text documents that don't need OCR

**OCR Quality Filters**:
- **High Confidence**: OCR confidence > 90%
- **Medium Confidence**: OCR confidence 70-90%
- **Low Confidence**: OCR confidence < 70%

## Search Interface

### Global Search Bar

**Location**: Available in the header on all pages
**Features**:
- **Real-time suggestions**: Shows results as you type
- **Quick results**: Top 5 matches with snippets
- **Fast navigation**: Direct access to documents
- **Search history**: Recent searches for quick access

**Usage**:
1. Click on the search bar in the header
2. Start typing your query
3. View instant suggestions and results
4. Click a result to navigate directly to the document

### Advanced Search Page

**Location**: Dedicated search page with full interface
**Features**:
- **Multiple search modes**: Toggle between search types
- **Filter sidebar**: All filtering options in one place
- **Result options**: Sorting, pagination, view modes
- **Export capabilities**: Export search results

**Interface Sections**:

#### Search Input Area
- **Query builder**: Visual query construction
- **Mode selector**: Choose search type (simple, phrase, fuzzy, boolean)
- **Suggestions**: Auto-complete and query recommendations

#### Filter Sidebar
- **File type filters**: Checkboxes for different formats
- **Date range picker**: Calendar interface for date selection
- **Size sliders**: Visual size range selection
- **Label selector**: Hierarchical label browser
- **Source filters**: Filter by upload source

#### Results Area
- **Sort options**: Relevance, date, filename, size
- **View modes**: List view, grid view, detail view
- **Pagination**: Navigate through result pages
- **Export options**: CSV, JSON export of results

### Search Results

#### Result Display Elements

**Document Cards**:
- **Filename**: Primary document identifier
- **Snippet**: Highlighted text excerpt showing search matches
- **Metadata**: File size, type, upload date, labels
- **Relevance Score**: Numerical relevance ranking
- **Quick Actions**: Download, view, edit labels

**Highlighting**:
- **Search terms**: Highlighted in yellow
- **Context**: Surrounding text for context
- **Multiple matches**: All instances highlighted
- **Snippet length**: Configurable in user settings

#### Result Sorting

**Sort Options**:
- **Relevance**: Best matches first (default)
- **Date**: Newest or oldest first
- **Filename**: Alphabetical order
- **Size**: Largest or smallest first
- **Score**: Highest search score first

**Secondary Sorting**:
- Apply secondary criteria when primary sort values are equal
- Example: Sort by relevance, then by date

### Search Configuration

#### User Preferences

**Search Settings** (accessible via Settings → Search):
- **Results per page**: 10, 25, 50, 100
- **Snippet length**: 100, 200, 300, 500 characters
- **Fuzzy threshold**: Sensitivity for approximate matching
- **Default sort**: Preferred default sorting option
- **Search history**: Enable/disable query history

#### Search Behavior
- **Auto-complete**: Enable search suggestions
- **Real-time search**: Search as you type
- **Search highlighting**: Highlight search terms in results
- **Context snippets**: Show surrounding text in results

## Search Optimization

### Query Optimization

#### Best Practices

1. **Use Specific Terms**: More specific queries yield better results
   ```
   Good: "quarterly sales report Q1"
   Poor: "document"
   ```

2. **Combine Search Modes**: Use appropriate mode for your needs
   ```
   Exact phrases: "status update"
   Flexible terms: project~
   Complex logic: (budget OR financial) AND 2024
   ```

3. **Leverage Filters**: Combine text search with filters
   ```
   Query: budget
   Filters: Type = PDF, Date = This Quarter, Label = Finance
   ```

4. **Use Field Search**: Target specific document aspects
   ```
   filename:invoice date:2024
   content:"project milestone" label:important
   ```

### Performance Tips

#### Efficient Searching

1. **Start Broad, Then Narrow**: Begin with general terms, then add filters
2. **Use Filters Early**: Apply filters before complex text queries
3. **Avoid Wildcards at Start**: `*report` is slower than `report*`
4. **Combine Short Queries**: Use multiple short terms rather than long phrases

#### Search Index Optimization

The search system automatically optimizes for:
- **Frequent Terms**: Common words are indexed for fast retrieval
- **Document Updates**: New documents are indexed immediately
- **Language Support**: Multi-language stemming and analysis
- **Cache Management**: Frequent searches are cached

### OCR Search Optimization

#### Handling OCR Text

OCR-extracted text may contain errors that affect search:

**Strategies**:
1. **Use Fuzzy Search**: Handle OCR errors with approximate matching
2. **Try Variations**: Search for common OCR mistakes
3. **Use Context**: Include surrounding words for better matches
4. **Check Original**: Compare with original document when possible

**Common OCR Issues**:
- **Character confusion**: "m" vs "rn", "cl" vs "d"
- **Word boundaries**: "some thing" vs "something"
- **Special characters**: Missing or incorrect punctuation

**Optimization Examples**:
```
# Original: "invoice"
# OCR might produce: "irwoice", "invoce", "mvoice"
# Solution: Use fuzzy search
invoice~

# Or search for context
"invoice number" OR "irwoice number" OR "invoce number"
```

## Saved Searches

### Creating Saved Searches

1. **Build Your Query**: Create a search with desired parameters
2. **Test Results**: Verify the search returns expected documents
3. **Save Search**: Click "Save Search" button
4. **Name Search**: Provide descriptive name
5. **Configure Options**: Set update frequency and notifications

### Managing Saved Searches

**Saved Search Features**:
- **Quick Access**: Available in sidebar or dashboard
- **Automatic Updates**: Results update as new documents are added
- **Shared Access**: Share searches with other users (future feature)
- **Export Options**: Export results automatically

**Search Organization**:
- **Categories**: Group related searches
- **Favorites**: Mark frequently used searches
- **Recent**: Quick access to recently used searches

### Smart Collections

Saved searches that automatically include new documents:

**Examples**:
- **"This Month's Reports"**: `type:pdf AND content:report AND date:this-month`
- **"Pending Review"**: `label:"needs review" AND -label:completed`
- **"High Priority Items"**: `label:(urgent OR critical OR "high priority")`

## Search Analytics

### Search Performance Metrics

**Available Metrics**:
- **Query Performance**: Average search response times
- **Popular Searches**: Most frequently used search terms
- **Result Quality**: Click-through rates and user engagement
- **Search Patterns**: Common search behaviors and trends

### User Search History

**History Features**:
- **Recent Searches**: Quick access to previous queries
- **Search Suggestions**: Based on search history
- **Query Refinement**: Improve searches based on past patterns
- **Export History**: Download search history for analysis

## API Search

### Basic Search API

```bash
GET /api/search?query=invoice&limit=20
Authorization: Bearer <jwt_token>
```

**Query Parameters**:
- `query`: Search query string
- `limit`: Number of results (default: 50, max: 100)
- `offset`: Pagination offset
- `sort`: Sort order (relevance, date, filename, size)

### Advanced Search API

```bash
POST /api/search/advanced
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "query": "budget report",
  "mode": "phrase",
  "filters": {
    "file_types": ["pdf", "docx"],
    "labels": ["Q1 2024", "Finance"],
    "date_range": {
      "start": "2024-01-01",
      "end": "2024-03-31"
    },
    "size_range": {
      "min": 1048576,
      "max": 52428800
    }
  },
  "options": {
    "fuzzy_threshold": 0.8,
    "snippet_length": 200,
    "highlight": true
  }
}
```

### Search Response Format

```json
{
  "results": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "filename": "Q1_Budget_Report.pdf",
      "snippet": "The quarterly budget report shows a <mark>10% increase</mark> in revenue...",
      "score": 0.95,
      "highlights": ["budget", "report"],
      "metadata": {
        "size": 2048576,
        "type": "application/pdf",
        "uploaded_at": "2024-01-15T10:30:00Z",
        "labels": ["Q1 2024", "Finance", "Budget"],
        "source": "WebDAV Sync"
      }
    }
  ],
  "total": 42,
  "limit": 20,
  "offset": 0,
  "query_time": 0.085
}
```

## Troubleshooting

### Common Search Issues

#### No Results Found

**Possible Causes**:
1. **Typos**: Check spelling in search query
2. **Too Specific**: Query might be too restrictive
3. **Wrong Mode**: Using exact search when fuzzy would be better
4. **Filters**: Remove filters to check if they're excluding results

**Solutions**:
1. **Simplify Query**: Start with broader terms
2. **Check Spelling**: Use fuzzy search for typo tolerance
3. **Remove Filters**: Test without date, type, or label filters
4. **Try Synonyms**: Use alternative terms for the same concept

#### Irrelevant Results

**Possible Causes**:
1. **Too Broad**: Query matches too many unrelated documents
2. **Common Terms**: Using very common words that appear everywhere
3. **Wrong Mode**: Using fuzzy when exact match is needed

**Solutions**:
1. **Add Specificity**: Include more specific terms or context
2. **Use Filters**: Add file type, date, or label filters
3. **Phrase Search**: Use quotes for exact phrases
4. **Boolean Logic**: Use AND/OR/NOT for better control

#### Slow Search Performance

**Possible Causes**:
1. **Complex Queries**: Very complex boolean queries
2. **Large Result Sets**: Queries matching many documents
3. **Wildcard Overuse**: Starting queries with wildcards

**Solutions**:
1. **Simplify Queries**: Break complex queries into simpler ones
2. **Add Filters**: Use filters to reduce result set size
3. **Avoid Leading Wildcards**: Use `term*` instead of `*term`
4. **Use Pagination**: Request smaller result sets

### OCR Search Issues

#### OCR Text Not Searchable

**Symptoms**: Can't find text that's visible in document images
**Solutions**:
1. **Check OCR Status**: Verify OCR processing completed
2. **Retry OCR**: Manually retry OCR processing
3. **Use Fuzzy Search**: OCR might have character recognition errors
4. **Check Language Settings**: Ensure correct OCR language is configured

#### Poor OCR Search Quality

**Symptoms**: Fuzzy search required for most queries on scanned documents
**Solutions**:
1. **Improve Source Quality**: Use higher resolution scans (300+ DPI)
2. **OCR Language**: Verify correct language setting for documents
3. **Image Enhancement**: Enable OCR preprocessing options
4. **Manual Correction**: Consider manual text correction for important documents

### Search Configuration Issues

#### Settings Not Applied

**Symptoms**: Search settings changes don't take effect
**Solutions**:
1. **Reload Page**: Refresh browser to apply settings
2. **Clear Cache**: Clear browser cache and cookies
3. **Check Permissions**: Ensure user has permission to modify settings
4. **Database Issues**: Check if settings are being saved to database

#### Filter Problems

**Symptoms**: Filters not working as expected
**Solutions**:
1. **Clear All Filters**: Reset filters and apply one at a time
2. **Check Filter Logic**: Ensure AND/OR logic is correct
3. **Label Validation**: Verify labels exist and are spelled correctly
4. **Date Format**: Ensure dates are in correct format

## Next Steps

- Explore [labels and organization](labels-and-organization.md) for better search categorization
- Set up [sources](sources-guide.md) for automatic content ingestion
- Review [user guide](user-guide.md) for general search tips
- Check [API reference](api-reference.md) for programmatic search integration
- Configure [OCR optimization](dev/OCR_OPTIMIZATION_GUIDE.md) for better text extraction