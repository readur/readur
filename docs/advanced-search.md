# Advanced Search Guide

This guide describes exactly what Readur's search does: which text it looks at, the four search modes, the filters you can combine with them, and the search API. Every example query and request in this guide is run by Readur's test suite (`tests/integration_search_docs_examples_tests.rs`), so the results described here are the results you get.

## Table of Contents

- [What Search Looks At](#what-search-looks-at)
- [Search Modes](#search-modes)
- [Filters](#filters)
- [Sorting and Paging](#sorting-and-paging)
- [Search API](#search-api)
- [Troubleshooting](#troubleshooting)

## What Search Looks At

A search query is matched against a document's **text**: the text extracted from the file (`content`) together with its OCR text, as one piece of text. Matching uses PostgreSQL full-text search with the English configuration, so words are stemmed (`invoices` finds `invoice`, `approved` finds `approve`) and common English stop words (`the`, `for`, `and`, ...) are ignored.

The query does **not** search file names, labels, file types, sources, sizes or dates, and there is no field syntax for them: `filename:invoice`, `label:urgent`, `type:pdf`, `source:webdav`, `size:>10MB` or `date:2024` are searched as ordinary words and will not do what they look like. To narrow a search by file type, label, OCR status, source or date, use the [filters](#filters); to find a file by name, sort the library by file name.

You can search with an empty query when at least one filter is set; otherwise a query needs at least 2 characters (and at most 1000).

## Search Modes

Choose the mode with the mode switch in the library's search help (the **?** button next to the search box), with `search_mode` on `GET /api/search/enhanced`, or with `search_mode` on the MCP `enhanced_search` tool. The default is **simple**.

| Mode (UI name) | `search_mode` | Matches documents whose text |
|----------------|---------------|------------------------------|
| All words | `simple` | contains every word of the query, anywhere, in any order |
| Exact phrase | `phrase` | contains the query's words next to each other, in order |
| Operators | `boolean` | satisfies the query's AND / OR / NOT / phrase / prefix expression |
| Similar spelling | `fuzzy` | as a whole is spelled similarly to the query |

### Simple Search (All Words)

Every word of the query must appear somewhere in the document's text. Words are stemmed and stop words are dropped, but a word must match a whole (stemmed) word: there is no partial matching, typo tolerance or synonym expansion. Punctuation and operator symbols have no meaning in this mode.

```
budget 2024                          # Both words, anywhere: finds "annual budget for 2024 approved"
budgets approved                     # Stemmed: finds the same document
the invoice                          # "the" is a stop word: finds every document mentioning "invoice"
inv                                  # No partial matching: finds nothing, although "invoice" exists
contract -draft                      # Operators are ignored: finds the document with both "contract" and "draft"
```

### Phrase Search (Exact Phrase)

The query's words must appear next to each other and in the given order (stop words in the query are dropped, and words are stemmed as in simple mode). Double quotes are not needed and are ignored.

```
total due                            # Finds "... 17 total due 300", but not "due total in reverse"
due total                            # Finds "due total in reverse" only
"total due"                          # Quotes are ignored: the same as total due
annual budget                        # Finds both annual budget documents
```

### Fuzzy Search (Similar Spelling)

Fuzzy mode compares the query with the document's **whole text** using trigram similarity (PostgreSQL `pg_trgm`), and finds documents whose similarity is above 0.3. The threshold is fixed: the fuzzy search threshold in the user settings is stored but not applied to search. Results are ranked by similarity.

Because the comparison is with the entire text, fuzzy search finds short documents whose text is close to the query even with typos, but a word inside a longer text adds little to the similarity: a misspelled word does not find a long document that contains the correct word. For long documents, use simple mode, or an Operators-mode prefix such as `inv*`.

```
waranty card                         # Finds a document whose text is "warranty card"
warranty                             # Finds it as well: the query is most of its text
invoce                               # Finds nothing: each invoice's text is much longer than the query
```

### Boolean Search (Logical Operators)

**Best for**: Complex queries with multiple conditions and precise control

Choose the **Operators** mode in the library search (or pass `search_mode=boolean` to `GET /api/search/enhanced`, or `search_mode: "boolean"` to the MCP `enhanced_search` tool). Readur parses the query and turns it into a PostgreSQL `to_tsquery`, so the operators below are the only syntax with a special meaning; everything else is a search word.

| Syntax | Meaning |
|--------|---------|
| `budget 2024`, `budget & 2024`, `budget AND 2024` | Both words. Words written next to each other are joined with AND |
| `invoice \| receipt`, `invoice OR receipt` | Either word |
| `contract !draft`, `contract NOT draft`, `contract -draft` | Leave a word out. A minus only negates when it is attached to what follows: `e-mail` is one word, and the lone minus in `contract - draft` is ignored |
| `(budget \| financial) & 2024` | Parentheses group. AND binds tighter than OR, so `invoice \| receipt paid` means `invoice \| (receipt & paid)` |
| `"total due"` | Exact phrase: the words in this order, next to each other. `-"draft copy"` and `NOT (draft \| copy)` leave out a phrase or a group |
| `inv*`, `inv:*` | Prefix: every word starting with `inv` (invoice, inventory) |
| `total <-> due`, `invoice <2> 17` | Followed by: `due` right after `total`, or `17` two words after `invoice` |

`AND`, `OR` and `NOT` work in any case (`and`, `Or`); as search words they are stop words that could never match anyway. Words are stemmed, so `invoices` also finds `invoice`. There is no `NEAR`, `XOR`, leading wildcard (`*report`) or single-character wildcard (`doc?ment`): those characters are treated as part of the word or dropped.

**Examples**:
```
budget 2024                          # Both "budget" and "2024"
budget AND 2024                      # The same, with the keyword
invoice | receipt                    # Either "invoice" or "receipt"
invoice OR receipt                   # The same, with the keyword
contract !draft                      # "contract" but not "draft"
contract NOT draft                   # The same, with the keyword
contract -draft                      # The same, with a leading minus
(budget | financial) & 2024          # Either word, and "2024"
marketing (campaign OR strategy)     # "marketing" and either "campaign" or "strategy"
invoice | receipt paid               # "invoice", or both "receipt" and "paid"
"total due"                          # The exact phrase "total due"
inv*                                 # Words starting with "inv"
total <-> due                        # "due" directly after "total"
invoice <2> 17                       # "17" two words after "invoice"
```

**Advanced Boolean Examples**:
```
# Completed project documents that are not drafts
project (final | completed | approved) !draft

# Financial documents, leaving out anything personal
(invoice OR receipt OR budget) NOT personal

# Signed or approved purchase orders, leaving out cancelled ones
"purchase order" (approved | signed) -cancel*
```

**Malformed queries**: a boolean search never fails because of its syntax. Operators with nothing to act on are ignored, empty groups are dropped, and an unclosed parenthesis or quote is closed at the end of the query:
```
invoice &                            # Same as: invoice
(invoice | receipt                   # Same as: (invoice | receipt)
"total due                           # Same as: "total due"
```

### Wildcard Search

In boolean mode, a trailing `*` (or `:*`) matches every word starting with what comes before it:

```
proj*           # Matches "project", "projects", "projection"
```

Leading wildcards (`*report`) and single-character wildcards (`doc?ment`) are not supported. For approximate matches, use fuzzy search.

### Exclusion Operators

In boolean mode, `!`, `NOT` or an attached `-` leave a word, phrase or group out:

```
invoice -draft                 # Invoices but not drafts
budget NOT personal            # Budget documents excluding personal
proposal !(archive | old)      # Proposals mentioning neither "archive" nor "old"
```

Simple mode searches for all the words you type and ignores these operators.

## Filters

Filters narrow a search (or the whole library, with no query) by properties of the document rather than its text. They are combined with AND: a document must pass every filter that is set. Within one filter, a document must match any of the listed values. In the library, filters are in the filter strip above the document list; in the API they are query parameters of `GET /api/search` and `GET /api/search/enhanced`.

| Filter | API parameter | Values |
|--------|---------------|--------|
| File type | `mime_types` | Comma-separated MIME types, e.g. `application/pdf,image/png`. The library's type groups (PDF, images, ...) expand to these |
| Labels | `label_ids` | Comma-separated label IDs; matches documents carrying any of them. An invalid ID is a 400 error |
| Labels by name | `tags` | Comma-separated label names (exact, case-sensitive) |
| OCR status | `ocr_status` | One of `pending` (also documents without a status), `processing`, `completed`, `failed` |
| Source | `source_ids` | Comma-separated source (connection) IDs |
| Source kind | `source_types` | Comma-separated source types such as `webdav`, `local_folder`, `s3`; `direct_upload` also matches every document that did not come from a source |
| Added between | `created_from`, `created_to` | RFC 3339 timestamps, both inclusive. `created_from` may not be later than `created_to` |

There are no size filters, and no filters on modification or access time.

## Sorting and Paging

Without a sort, `GET /api/search/enhanced` (and the library, when **Relevance** is chosen) orders results by relevance: full-text rank in simple, phrase and Operators mode, similarity in fuzzy mode, with ties broken by newest first. `GET /api/search` always orders newest first unless a sort is given.

- `sort_by`: `created_at`, `updated_at`, `filename`, `file_size`, `ocr_status`, `mime_type` or `ocr_confidence`
- `sort_order`: `asc` or `desc` (default `desc`)
- `limit`: results per page, 1 to 1000 (default 25)
- `offset`: results to skip (default 0)

## Search API

All search endpoints take a bearer token and only return documents the caller may see (their own; administrators see every document).

| Endpoint | What it does |
|----------|--------------|
| `GET /api/search` | Simple-mode search, newest first by default, without snippets or ranking |
| `GET /api/search/enhanced` | Search in any mode, ranked by relevance, with highlighted snippets |
| `GET /api/search/facets` | The caller's MIME types and label names, with document counts, for building filters |
| `GET /api/search/timeline` | The number of matching documents per month (`YYYY-MM`), for the same parameters as a search |

Parameters of `GET /api/search` and `GET /api/search/enhanced`, besides the filters and sorting above:

- `query`: the search text (see [What Search Looks At](#what-search-looks-at))
- `search_mode`: `simple` (default), `phrase`, `boolean` or `fuzzy`; `GET /api/search` always searches in simple mode
- `include_snippets`: whether to return snippets (default `true`, enhanced search only)
- `snippet_length`: snippet size in characters, 1 to 2000 (default 200)

Spaces and other reserved characters in a parameter value must be URL-encoded (`%20` for a space).

### Example Requests

These requests run against a library of four documents: a PDF with the text "invoice for march" (added 2024-03-10, OCR completed, labelled Finance), a PNG "scanned invoice" (2024-06-01, OCR pending, from a WebDAV source), a text file "contract signed" (2025-01-05, OCR failed, labelled Finance, from a local folder) and a PDF "contract draft" (2023-11-20, OCR processing, from a WebDAV source).

```
GET /api/search/enhanced?query=invoice                                    # Both invoices
GET /api/search/enhanced?query=invoice&mime_types=application/pdf         # Only the PDF invoice
GET /api/search/enhanced?query=contract&ocr_status=failed                 # The signed contract
GET /api/search/enhanced?label_ids=<label-id>                             # No query, only a filter: everything labelled Finance
GET /api/search/enhanced?query=contract&tags=Finance                      # The signed contract, filtering by label name
GET /api/search/enhanced?query=contract&source_types=webdav               # The draft contract
GET /api/search/enhanced?query=invoice&created_from=2024-05-01T00:00:00Z  # The PNG invoice, added after May 1st 2024
GET /api/search/enhanced?query=contract%20!draft&search_mode=boolean      # Operators mode: the signed contract
GET /api/search/enhanced?query=contract&sort_by=created_at&sort_order=asc # Both contracts, oldest first
GET /api/search?query=invoice&limit=1                                     # Basic search, newest first: the PNG invoice
```

### Response

Both search endpoints answer with this shape (one document shown, abbreviated values):

```json
{
  "documents": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "filename": "march_invoice.pdf",
      "original_filename": "march_invoice.pdf",
      "file_size": 2048,
      "mime_type": "application/pdf",
      "tags": [],
      "labels": [],
      "created_at": "2024-03-10T09:00:00Z",
      "updated_at": "2024-03-10T09:00:00Z",
      "source_id": null,
      "source_type": null,
      "has_ocr_text": true,
      "ocr_confidence": 95.5,
      "ocr_word_count": 3,
      "ocr_processing_time_ms": 1200,
      "ocr_status": "completed",
      "search_rank": 0.06,
      "snippets": [
        {
          "text": "invoice for march",
          "start_offset": 0,
          "end_offset": 17,
          "highlight_ranges": [{ "start": 0, "end": 7 }]
        }
      ]
    }
  ],
  "total": 1,
  "query_time_ms": 4,
  "suggestions": []
}
```

`total` is the number of matching documents across all pages. While a document is being OCRed, it also carries `ocr_progress_current` and `ocr_progress_total` (pages done and pages in total). `search_rank` and `snippets` are only filled by enhanced search. `suggestions` are alternative queries (the query as a phrase, and with a trailing `*`) for queries longer than three characters.

## Troubleshooting

### No Results Found

- **Check the mode.** Phrase mode needs the words adjacent and in order; simple mode needs every word; fuzzy mode only finds documents whose whole text is close to the query.
- **Check the filters.** Filters combine with AND, so one restrictive filter hides everything else. Clear them and add them back one at a time.
- **Whole words only.** Simple and phrase mode do not match parts of words: search `invoice`, not `inv`, or use the Operators-mode prefix `inv*`.
- **Field syntax is not supported.** `filename:`, `label:`, `type:` and similar are searched as plain words; use filters instead.
- **The text may not be extracted yet.** A document is only found by its text once OCR has completed. Filter by OCR status `pending`, `processing` or `failed` to find documents still without text, and retry OCR from the document's page if it failed.

### Too Many Results

- Add words (simple mode requires all of them) or use phrase mode.
- In Operators mode, leave out unwanted words with `!word`, `NOT word` or `-word`.
- Combine the query with filters, such as a file type, label or date range.

### OCR Errors in the Text

Scanned documents can contain misrecognised words ("irwoice" for "invoice"). Fuzzy mode only helps for short documents; for longer ones, search for another word that was recognised correctly, use an Operators-mode prefix (`inv*`), or improve the scan quality and retry OCR.

## Next Steps

- Explore [labels and organization](labels-and-organization.md) for better search categorization
- Set up [sources](sources-guide.md) for automatic content ingestion
- Review the [user guide](user-guide.md) for general search tips
- Check the [API reference](api-reference.md) for programmatic search integration
