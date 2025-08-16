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

The simple search mode includes powerful **Auto-stemming** capabilities, where "running" automatically matches variations like "run", "runs", and "runner". This linguistic intelligence extends to **Fuzzy tolerance**, correcting common typos so "recieve" successfully finds "receive".

**Partial matching** allows shorter queries to find longer terms - typing "doc" will match "document", "documentation", and other related terms. The system's **Relevance ranking** ensures the most pertinent results appear first, using sophisticated algorithms that consider term frequency, document importance, and contextual relevance. Additionally, the search engine applies **Synonym expansion** for common terms and includes **Stop word filtering** to improve search quality.

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

Fuzzy search can be fine-tuned through **Threshold adjustment** in your user settings, allowing you to configure how sensitive the system is to spelling variations. This is particularly important because **Language-specific** requirements vary - for instance, Germanic languages might need different thresholds than Romance languages due to their structural differences.

The system provides **OCR optimization** with higher tolerance settings specifically for OCR-processed documents, which often contain character recognition errors. You can also configure **Context-aware matching** that adjusts fuzzy thresholds based on surrounding words, and set up **Custom dictionaries** for industry-specific terminology that might not be in standard dictionaries.

### Boolean Search (Logical Operators)

**Best for**: Complex queries with multiple conditions and precise control

**Operators**:

Boolean search provides precise control through logical operators. The **AND** operator requires both terms to be present in matching documents, perfect for narrowing results to specific criteria. Conversely, the **OR** operator broadens your search by accepting documents containing either term.

For exclusion logic, the **NOT** operator removes documents containing unwanted terms from your results. To handle complex queries, **Parentheses** allow you to group conditions and control the order of operations. The system also supports **NEAR** operators for proximity searches and **XOR** for exclusive or logic, giving you complete control over your search logic.

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

The file type filter supports a comprehensive range of formats. **Documents** include standard formats like PDF, DOC, DOCX, TXT, and RTF, covering most text-based files you'll encounter in a business environment.

**Images** are fully supported with common formats including PNG, JPG, JPEG, TIFF, BMP, and GIF - all of which can be processed through OCR for text extraction. For data analysis, **Spreadsheets** in XLS, XLSX, and CSV formats are searchable, with the system able to extract and index tabular data.

**Presentations** in PPT and PPTX formats are also indexed, including slide text and speaker notes. Beyond these categories, the system handles **Archive files** (ZIP, RAR, 7Z), **Email formats** (EML, MSG), and **eBook formats** (EPUB, MOBI) for comprehensive document management.

**Filter Interface**:

The filtering interface provides multiple ways to narrow your search by file type. You can use **Checkbox Filters** to quickly select or deselect multiple file types with a single click, making it easy to focus on specific document formats.

For broader categorization, **MIME Type Groups** let you filter by general categories like "all documents" or "all images" without selecting individual formats. The system also supports **Custom Extensions** where you can add specific file extensions that might be unique to your organization. Additionally, there's a **Quick Toggle** feature for common combinations and **Saved Filter Sets** that remember your frequently used filter configurations.

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

File size filtering uses intuitive categories to help you find documents of specific sizes. **Small** files under 1MB typically include text documents, simple PDFs, and low-resolution images - perfect for quick reference materials.

The **Medium** category (1MB - 10MB) encompasses most business documents, including formatted reports, presentations, and high-quality images. **Large** files (10MB - 50MB) often contain detailed technical documentation, multi-page scanned documents, or multimedia presentations.

Documents in the **Very Large** category (over 50MB) usually include comprehensive manuals, video content, or high-resolution design files. The system also provides **Micro** (<100KB) and **Gigantic** (>500MB) categories for edge cases, along with custom size range inputs for precise filtering.

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

Documents can enter your system through various channels, each tracked separately for filtering. **Manual Upload** identifies documents that users have uploaded directly through the web interface or API, typically one-off additions or user-generated content.

**WebDAV Sync** marks documents automatically synchronized from WebDAV servers like Nextcloud or ownCloud, often representing shared team resources. The **Local Folder** source indicates documents ingested from watched directories on the server, useful for automated workflows and bulk imports.

**S3 Sync** identifies documents pulled from Amazon S3 or compatible cloud storage, commonly used for large-scale document repositories. The system also tracks **Email Attachments**, **API Uploads**, and **Migration Imports** as distinct sources for complete visibility into document origins.

**Source-Specific Filters**:
```
source:webdav                  # WebDAV synchronized documents
source:manual                  # Manually uploaded documents
source:"My Nextcloud"          # Specific named source
```

### OCR Status Filters

Filter by OCR processing status:

**Status Options**:

OCR status filtering helps you manage document processing states effectively. Documents marked as **Completed** have been successfully processed with text extraction finished and content indexed for searching.

The **Pending** status indicates documents still waiting in the OCR queue, which might need attention if the queue is backing up. **Failed** status highlights documents where OCR processing encountered errors - these might need manual review or reprocessing with different settings.

Documents marked **Not Applicable** are text-based files that don't require OCR processing, such as native PDFs with embedded text or plain text files. The system also tracks **In Progress** for actively processing documents, **Partial** for documents with mixed success, and **Skipped** for documents excluded by configuration rules.

**OCR Quality Filters**:

Filter documents based on OCR extraction confidence levels to focus on quality. **High Confidence** documents (over 90% confidence) contain reliably extracted text suitable for critical searches and automated workflows.

**Medium Confidence** results (70-90%) represent acceptable quality with occasional errors, typically from slightly degraded originals or handwritten sections. Documents with **Low Confidence** (below 70%) may contain significant extraction errors and often benefit from manual review or reprocessing.

The system provides additional quality indicators including **Language Match** confidence, **Layout Preservation** quality, and **Character Recognition** accuracy scores, helping you identify documents that might need attention or alternative processing strategies.

## Search Interface

### Global Search Bar

**Location**: Available in the header on all pages
**Features**:
- **Real-time suggestions**: Shows results as you type
- **Quick results**: Top 5 matches with snippets
- **Fast navigation**: Direct access to documents
- **Search history**: Recent searches for quick access

**Usage**:

The global search bar provides instant access to your documents from anywhere in the application. Simply click on the search bar located in the header to activate it and place your cursor ready for input.

As you start typing your query, the system immediately begins processing, showing instant suggestions based on your input and search history. The dropdown displays the top matching results with highlighted snippets, giving you a preview of each document's relevance.

Clicking any result navigates directly to the document viewer, while pressing Enter takes you to the full search results page with all matches. The interface also supports keyboard shortcuts - use arrow keys to navigate suggestions, Tab to autocomplete, and Escape to close the search dropdown.

### Advanced Search Page

**Location**: Dedicated search page with full interface
**Features**:
- **Multiple search modes**: Toggle between search types
- **Filter sidebar**: All filtering options in one place
- **Result options**: Sorting, pagination, view modes
- **Export capabilities**: Export search results

**Interface Sections**:

#### Search Input Area

The search input area serves as your command center for building sophisticated queries. The **Query builder** provides visual query construction tools, including drag-and-drop operators and clickable filter tags that make complex searches accessible to all users.

A **Mode selector** lets you instantly switch between search types - simple for everyday use, phrase for exact matches, fuzzy for typo tolerance, and boolean for complex logic. The **Suggestions** system offers intelligent auto-complete based on your search history, popular queries, and document content.

Additionally, the interface includes a **Query validator** that checks syntax in real-time, **Search templates** for common query patterns, and a **Query history** dropdown for quick access to recent searches.

#### Filter Sidebar

The filter sidebar consolidates all filtering options in an intuitive, collapsible panel. **File type filters** present checkboxes for different formats, organized by category with select-all options and quick presets for common combinations.

The **Date range picker** offers a calendar interface for precise date selection, including preset ranges like "Last 7 days" or "This quarter" for convenience. **Size sliders** provide visual range selection with logarithmic scaling to handle the wide range of file sizes effectively.

For organization, the **Label selector** displays your hierarchical label structure in a tree view, supporting multi-select with AND/OR logic. **Source filters** let you filter by upload source with usage statistics shown for each source. The sidebar also includes **OCR status filters** and **Custom metadata fields** for advanced filtering capabilities.

#### Results Area

The results area adapts to your preferred way of viewing search results. **Sort options** include relevance scoring, upload date, filename alphabetically, and file size, with secondary sort criteria available for tie-breaking.

**View modes** cater to different preferences - list view for maximum information density, grid view for visual browsing of documents with thumbnails, and detail view for in-depth document examination without leaving search results. The **Pagination** controls offer flexible navigation through result pages, with options for 10, 25, 50, or 100 results per page.

For data portability, **Export options** allow you to download search results in CSV format for spreadsheet analysis or JSON for programmatic processing. The results area also features **Bulk actions** for operating on multiple documents, **Quick preview** on hover, and **Keyboard navigation** for power users.

### Search Results

#### Result Display Elements

**Document Cards**:

Each search result is presented as an information-rich card designed for quick evaluation. The **Filename** serves as the primary identifier, displayed prominently with file type icon and extension for immediate recognition.

A **Snippet** shows highlighted text excerpts where your search terms appear, with intelligent context extraction ensuring you see the most relevant portions of each document. The **Metadata** section efficiently displays essential information including file size, document type, upload date, and applied labels in a scannable format.

The **Relevance Score** provides a numerical ranking (0-100) helping you understand why certain documents rank higher than others. **Quick Actions** are available on hover or click, offering immediate access to download, view, or edit operations without leaving the search interface. Cards also display **OCR confidence**, **Source information**, and **Last modified** timestamps for complete context.

**Highlighting**:

Search result highlighting makes it easy to spot relevant content within documents. **Search terms** are prominently highlighted in yellow (or your chosen color scheme) throughout snippets and document previews, ensuring quick visual scanning.

The system includes sufficient **Context** around matched terms, showing surrounding sentences to help you understand the relevance without opening the document. When documents contain **Multiple matches**, all instances are highlighted with a match counter showing how many times your search terms appear.

**Snippet length** can be adjusted in user settings from compact 100-character excerpts to detailed 500-character passages based on your preference. The highlighting system also supports **Synonym highlighting** in different colors, **Phrase boundary markers** for exact matches, and **Fuzzy match indicators** showing approximate matches with confidence scores.

#### Result Sorting

**Sort Options**:

Flexible sorting options help you organize search results according to your needs. **Relevance** sorting (the default) uses sophisticated algorithms to place the best matches first, considering term frequency, document importance, and search context.

**Date** sorting can display newest or oldest documents first, essential for finding recent updates or historical documents. **Filename** ordering arranges results alphabetically, useful when you know partial filenames or want to group similar documents.

**Size** sorting helps identify the largest or smallest files, valuable for storage management or finding specific document types. Beyond these primary options, you can sort by **Score** for numerical relevance ranking, **Modification time** for recently edited documents, and **Access frequency** to surface popular documents.

**Secondary Sorting**:
- Apply secondary criteria when primary sort values are equal
- Example: Sort by relevance, then by date

### Search Configuration

#### User Preferences

**Search Settings** (accessible via Settings → Search):

Personalize your search experience through comprehensive user preferences. **Results per page** can be set to 10, 25, 50, or 100 items depending on your screen size and browsing preference, with the system remembering your choice across sessions.

**Snippet length** options range from concise 100-character excerpts to detailed 500-character passages, letting you balance information density with readability. The **Fuzzy threshold** slider adjusts sensitivity for approximate matching, particularly useful if you frequently search OCR documents or deal with technical terms.

Your **Default sort** preference ensures results always appear in your preferred order, whether that's relevance, date, or another criterion. **Search history** can be enabled or disabled based on privacy preferences and workflow needs. Additional settings include **Highlighting colors**, **Auto-suggestion delay**, and **Advanced mode defaults** for power users.

#### Search Behavior

Configure how the search system responds to your interactions for an optimized experience. **Auto-complete** functionality can be toggled to show search suggestions as you type, drawing from your history, popular queries, and document content to speed up query creation.

**Real-time search** enables instant result updates as you modify your query, perfect for exploratory searching where you're refining terms to find the right documents. This feature can be disabled if you prefer to complete your query before searching.

**Search highlighting** ensures your search terms stand out in results, with customizable colors and styles to match your visual preferences. **Context snippets** control how much surrounding text appears with matches, helping you evaluate relevance without opening documents. The system also offers **Spell checking** with automatic correction suggestions, **Search shortcuts** for frequently used queries, and **Predictive filtering** that suggests relevant filters based on your query.

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

#### Efficient Searching

Optimize your search strategy for best performance and results. **Start Broad, Then Narrow** by beginning with general terms to gauge the document landscape, then progressively add filters and specific terms to refine results to exactly what you need.

**Use Filters Early** in your search process - applying file type, date, or label filters before complex text queries reduces the dataset size and speeds up text matching. This is particularly effective when you know the general characteristics of the documents you're seeking.

When using wildcards, **Avoid Wildcards at Start** of terms as `*report` requires scanning all terms in the index, while `report*` can use the index efficiently. This seemingly small difference can impact search speed significantly on large document collections.

**Combine Short Queries** strategically - multiple focused terms often work better than long phrases, as they're more flexible in matching and allow the relevance algorithm to work effectively. Additionally, consider using **field-specific searches** to target particular document attributes, and leverage **search templates** for commonly repeated queries.

#### Search Index Optimization

#### Search Index Optimization

The search system employs multiple automatic optimizations to ensure fast, accurate results. **Frequent Terms** that appear often in queries are specially indexed with optimized data structures for lightning-fast retrieval, reducing search latency for common queries.

**Document Updates** trigger immediate reindexing, ensuring new content becomes searchable within seconds of upload or modification. This real-time indexing eliminates the delays common in batch-processing systems.

Comprehensive **Language Support** includes language-specific stemming algorithms and analysis rules for over 20 languages, ensuring accurate search regardless of document language. The system automatically detects document language and applies appropriate processing.

**Cache Management** intelligently stores results from frequent searches, dramatically reducing response time for popular queries while managing memory usage efficiently. The system also performs **Index compaction** during low-usage periods, **Query optimization** through automatic rewriting, and **Distributed indexing** for large deployments to maintain consistent performance at scale.

### OCR Search Optimization

#### Handling OCR Text

OCR-extracted text may contain errors that affect search:

**Strategies**:
1. **Use Fuzzy Search**: Handle OCR errors with approximate matching
2. **Try Variations**: Search for common OCR mistakes
3. **Use Context**: Include surrounding words for better matches
4. **Check Original**: Compare with original document when possible

**Common OCR Issues**:

Understanding typical OCR errors helps you craft better searches for scanned documents. **Character confusion** frequently occurs with similar-looking letter combinations - "m" might be read as "rn", "cl" as "d", or "li" as "h", particularly in lower-quality scans.

**Word boundaries** present another challenge where OCR might incorrectly split or merge words, reading "something" as "some thing" or "can not" as "cannot". These errors are especially common with justified text or unusual fonts.

**Special characters** including punctuation, symbols, and diacritical marks often get misread or omitted entirely, turning "don't" into "dont" or "café" into "cafe". The system also commonly encounters **Case confusion** where uppercase I becomes lowercase l, **Number-letter swaps** like 0/O or 1/I/l, and **Ligature problems** where connected letters in certain fonts get misinterpreted.

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

Saved searches transform one-time queries into powerful ongoing tools. **Quick Access** from the sidebar or dashboard means your most important searches are always one click away, eliminating the need to recreate complex queries.

**Automatic Updates** ensure your saved searches stay current - as new documents matching your criteria are added to the system, they automatically appear in saved search results. This creates dynamic document collections without manual maintenance.

The **Shared Access** capability (coming soon) will allow you to share carefully crafted searches with team members, ensuring everyone uses consistent search criteria for common tasks. **Export Options** enable automatic result export on a schedule, perfect for regular reporting needs.

Saved searches also support **Change notifications** alerting you when new matches appear, **Version tracking** to see how results change over time, and **Search analytics** showing usage patterns and result quality metrics.

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

Comprehensive analytics help you understand and optimize search usage across your organization. **Query Performance** tracking shows average response times broken down by query complexity, time of day, and result set size, helping identify optimization opportunities.

**Popular Searches** analysis reveals the most frequently used search terms and queries, providing insights into what information users need most often. This data can inform document organization, labeling strategies, and training priorities.

**Result Quality** metrics including click-through rates, dwell time, and refinement patterns indicate whether searches are successfully connecting users with needed documents. Low engagement might suggest indexing issues or user training needs.

**Search Patterns** analysis uncovers common search behaviors, query refinement sequences, and feature usage trends. The system also tracks **Failed searches** with no results, **Search abandonment** rates, and **Filter usage** patterns to provide a complete picture of search effectiveness.

### User Search History

**History Features**:

Your search history becomes a powerful tool for improving search efficiency. **Recent Searches** provides instant access to previous queries through a dropdown menu, eliminating the need to retype complex searches and making it easy to revisit earlier research.

**Search Suggestions** leverage your personal search history along with successful searches from across the system to offer intelligent query recommendations as you type. These suggestions learn from your patterns over time, becoming more accurate and personalized.

**Query Refinement** tools analyze your search patterns to suggest improvements - if you frequently refine searches in similar ways, the system learns and suggests these refinements proactively. **Export History** functionality lets you download your complete search history for analysis, audit purposes, or migration to other systems.

The history system also includes **Collaborative filtering** to suggest searches based on similar users' patterns, **Temporal analysis** showing how your search needs change over time, and **Privacy controls** allowing you to clear or disable history tracking as needed.

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

#### No Results Found

**Possible Causes**:

When searches return no results, systematic troubleshooting usually reveals the issue. **Typos** in search queries are the most common cause - even small spelling errors can prevent matches if you're using exact or phrase search modes.

Your query might be **Too Specific**, combining multiple restrictive terms that no single document satisfies. This often happens when copying and pasting long phrases or using very technical terminology. Consider whether you're using the **Wrong Mode** - exact phrase searches won't find variations, while fuzzy search might be needed for OCR documents with recognition errors.

**Filters** can inadvertently exclude all results, especially when multiple filters combine with AND logic. Date ranges might be too narrow, or label filters might reference labels that have been renamed or deleted. It's also worth checking if **Permission restrictions** are limiting visible results, or if **Index lag** means recently uploaded documents aren't yet searchable.

**Solutions**:

Resolving "no results" issues requires a systematic approach to identify what's blocking matches. **Simplify Query** by starting with just one or two broad terms, then gradually add specificity once you confirm documents exist in your target area.

**Check Spelling** carefully, or switch to fuzzy search mode which tolerates typos and variations. This is particularly important for proper names, technical terms, or content extracted from OCR where errors are common.

**Remove Filters** systematically - temporarily disable all filters to see if results appear, then reapply them one at a time to identify which filter is too restrictive. Pay special attention to date ranges and source filters.

**Try Synonyms** and alternative phrasings for your concepts - what you call a "report" might be labeled as "summary", "analysis", or "review" in the actual documents. Additionally, consider searching for **Related terms** that often appear alongside your target content, use **Wildcard searches** to catch variations, and check the **Search scope** to ensure you're searching all available document fields.

#### Irrelevant Results

#### Irrelevant Results

**Possible Causes**:

Receiving irrelevant results often stems from queries that cast too wide a net. **Too Broad** queries using generic terms like "document" or "file" match nearly everything in your repository, burying relevant results in noise.

Searching with **Common Terms** that appear in most documents - words like "page", "date", or "company" - dilutes result relevance. These terms add little discriminatory value to your search. Using the **Wrong Mode** can also cause relevance issues; fuzzy search might match too many variations when you need exact phrase matching for specific terminology.

Other factors include **Missing context** where single words lack the surrounding terms that would clarify intent, **Outdated relevance** when old but highly-referenced documents outrank newer relevant ones, and **Language confusion** where multilingual content causes unexpected matches.

**Solutions**:

Improving result relevance requires refining your search strategy to be more targeted. **Add Specificity** by including additional context terms that distinguish your desired documents from others - instead of just "budget", try "budget 2024 marketing" for precision.

**Use Filters** aggressively to narrow the result set before text matching occurs. File type, date ranges, and label filters can eliminate large swaths of irrelevant documents, letting text search focus on a smaller, more relevant set.

**Phrase Search** with quotation marks ensures multi-word concepts stay together, preventing matches where terms appear separately in unrelated contexts. This is essential for finding specific titles, names, or technical phrases.

**Boolean Logic** provides surgical precision in defining what should and shouldn't appear in results. Combine AND to require multiple concepts, OR for alternatives, and NOT to exclude irrelevant documents. Beyond these techniques, consider **Field-specific searches** to target just filenames or content, **Relevance tuning** in search settings, and **Proximity operators** to require terms appear near each other.

#### Slow Search Performance

#### Slow Search Performance

**Possible Causes**:

Search performance can degrade for several reasons that are usually correctable. **Complex Queries** with deeply nested boolean logic, multiple wildcards, or extensive OR conditions require more processing time as the system evaluates numerous combinations.

**Large Result Sets** slow down both search execution and result rendering. When queries match thousands of documents, the system must score and sort all matches before displaying even the first page. **Wildcard Overuse**, particularly leading wildcards like `*report`, forces full index scans rather than efficient prefix matching.

Performance also suffers from **Fuzzy search overhead** with very low similarity thresholds, **Uncached queries** that are unique or rarely used, and **Resource contention** when multiple users run complex searches simultaneously.

**Solutions**:

Optimizing slow searches often involves adjusting your search strategy rather than waiting for results. **Simplify Queries** by breaking complex boolean expressions into multiple simpler searches, then combine results mentally or through saved searches.

**Add Filters** before text search to reduce the document pool being searched. Date ranges, file types, and source filters can eliminate 90% of documents before expensive text matching begins, dramatically improving speed.

**Avoid Leading Wildcards** which require examining every term in the index. Replace `*report` with `report*` or search for "report" without wildcards, using fuzzy matching if variation tolerance is needed.

**Use Pagination** effectively by requesting smaller result sets (25-50 results) rather than large sets (100+). Most relevant results appear early, so you rarely need to see everything at once. Additional optimizations include **Caching frequent searches** by saving them, **Scheduling complex searches** for off-peak hours, and **Using search templates** that are pre-optimized for common query patterns.

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