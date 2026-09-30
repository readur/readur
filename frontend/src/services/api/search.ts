export interface SearchRequest {
  query: string
  tags?: string  // Comma-separated label names (e.g., "important,work")
  mime_types?: string  // Comma-separated MIME types (e.g., "application/pdf,image/png")
  limit?: number
  offset?: number
  include_snippets?: boolean
  snippet_length?: number
  search_mode?: 'simple' | 'phrase' | 'fuzzy' | 'boolean'
}

export interface HighlightRange {
  start: number
  end: number
}

export interface SearchSnippet {
  text: string
  start_offset: number
  end_offset: number
  highlight_ranges: HighlightRange[]
}

export interface EnhancedDocument {
  id: string
  filename: string
  original_filename: string
  file_path: string
  file_size: number
  mime_type: string
  tags: string[]
  created_at: string
  updated_at: string
  user_id: string
  username?: string
  file_hash?: string
  original_created_at?: string
  original_modified_at?: string
  source_path?: string
  source_type?: string
  source_id?: string
  file_permissions?: number
  file_owner?: string
  file_group?: string
  source_metadata?: Record<string, any>
  has_ocr_text: boolean
  ocr_confidence?: number
  ocr_word_count?: number
  ocr_processing_time_ms?: number
  ocr_status?: string
  ocr_error?: string
  ocr_failure_reason?: string
  ocr_retry_count?: number
  ocr_completed_at?: string
  search_rank?: number
  snippets: SearchSnippet[]
}

export interface SearchResponse {
  documents: EnhancedDocument[]
  total: number
  query_time_ms: number
  suggestions: string[]
}

export interface FacetItem {
  value: string
  count: number
}

export interface SearchFacetsResponse {
  mime_types: FacetItem[]
  tags: FacetItem[]
}
