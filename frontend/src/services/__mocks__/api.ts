import { vi } from 'vitest'

// Mock axios instance
export const api = {
  defaults: { headers: { common: {} } },
  get: vi.fn(),
  post: vi.fn(),
  put: vi.fn(),
  delete: vi.fn(),
}

// Mock document service
export const documentService = {
  list: vi.fn(),
  getById: vi.fn(),
  getOcrText: vi.fn(),
  upload: vi.fn(),
  delete: vi.fn(),
  search: vi.fn(),
  enhancedSearch: vi.fn(),
  download: vi.fn(),
  getThumbnail: vi.fn(),
  getProcessedImage: vi.fn(),
  updateTags: vi.fn(),
  getFailedOcrDocuments: vi.fn(),
  getDuplicates: vi.fn(),
  retryOcr: vi.fn(),
  deleteLowConfidence: vi.fn(),
  getDocumentRetryHistory: vi.fn(),
  getRetryRecommendations: vi.fn(),
  getRetryStats: vi.fn(),
  bulkRetryOcr: vi.fn(),
}

// Re-export types that components might need
export interface Document {
  id: string
  filename: string
  original_filename: string
  file_size: number
  mime_type: string
  tags: string[]
  created_at: string
  updated_at?: string
  user_id?: string
  file_hash?: string
  has_ocr_text: boolean
  ocr_confidence?: number
  ocr_word_count?: number
  ocr_processing_time_ms?: number
  ocr_status?: string
  // New metadata fields
  original_created_at?: string
  original_modified_at?: string
  source_metadata?: Record<string, any>
}

export interface SearchRequest {
  query: string
  tags?: string[]
  mime_types?: string[]
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
  file_size: number
  mime_type: string
  tags: string[]
  created_at: string
  has_ocr_text: boolean
  ocr_confidence?: number
  ocr_word_count?: number
  ocr_processing_time_ms?: number
  ocr_status?: string
  search_rank?: number
  snippets: SearchSnippet[]
}

export interface SearchResponse {
  documents: EnhancedDocument[]
  total: number
  query_time_ms: number
  suggestions: string[]
}

export default api