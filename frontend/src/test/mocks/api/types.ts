/**
 * TypeScript types for the mock API framework
 * Extends and mirrors the main API types with additional mock-specific functionality
 */

// Instead of importing types that may not exist, define them locally
// This provides better isolation for the mock framework

// Basic API types needed for mocking
export interface Document {
  id: string
  filename: string
  original_filename?: string
  content?: string
  upload_date: string
  created_at?: string
  original_created_at?: string
  updated_at?: string
  user_id: string
  username?: string
  file_size: number
  mime_type: string
  ocr_text?: string
  ocr_confidence?: number
  has_ocr_text?: boolean
  ocr_status?: 'pending' | 'processing' | 'completed' | 'failed'
  ocr_word_count?: number
  ocr_processing_time_ms?: number
  thumbnail_path?: string
  processed_image_path?: string
  word_count?: number
  language?: string
  file_hash?: string
  file_path?: string
  source_type?: string
  source_path?: string
  source_metadata?: any
  tags?: string[]
}

export interface SearchResponse {
  documents: Document[]
  total: number
  query_time_ms: number
  suggestions?: string[]
}

export interface SearchRequest {
  query: string
  limit?: number
  offset?: number
  sort_by?: string
  sort_order?: 'asc' | 'desc'
  filters?: Record<string, any>
}

export interface EnhancedDocument extends Document {
  snippet?: string
  highlights?: HighlightRange[]
  score?: number
}

export interface HighlightRange {
  start: number
  end: number
  text: string
}

export interface SearchSnippet {
  text: string
  highlights: HighlightRange[]
}

export interface BulkOcrRetryResponse {
  success: boolean
  message: string
  retry_count: number
}

export interface QueueStats {
  pending_count: number
  processing_count: number
  completed_count: number
  completed_today?: number
  failed_count: number
  total_processed: number
  avg_wait_time_minutes?: number
  oldest_pending_minutes?: number
}

export interface OcrResponse {
  success: boolean
  text?: string
  confidence?: number
  error?: string
  document_id?: string
  filename?: string
  has_ocr_text?: boolean
}

export interface OcrRetryFailureReason {
  reason: string
  count: number
  avg_file_size_mb: number
  first_occurrence: string
  last_occurrence: string
}

export interface OcrRetryStatsResponse {
  total_retries: number
  successful_retries: number
  failed_retries: number
  retry_rate: number
  failure_reasons?: OcrRetryFailureReason[]
  file_types?: Record<string, any>
}

export interface SyncProgressInfo {
  source_id: string
  status: 'idle' | 'discovering' | 'processing' | 'completed' | 'error'
  current_step: string
  progress_percentage: number
  files_processed: number
  total_files: number
  current_file?: string
  estimated_time_remaining?: number
  phase?: string
  phase_description?: string
}

export interface SearchFacetsResponse {
  mime_types: string[]
  tags: string[]
}

export interface AvailableLanguagesResponse {
  languages: Array<{ code: string; name: string }>
}

export interface UserWatchDirectoryResponse {
  path: string
  enabled: boolean
}

// Mock configuration types
export interface MockConfig {
  delay?: number | 'infinite'
  shouldFail?: boolean
  errorCode?: number
  errorMessage?: string
  customResponse?: any
}

export interface MockScenario {
  name: string
  description: string
  config: MockConfig
  data?: any
}

// Enhanced types for mocking
export interface MockDocument extends Document {
  _mockId?: string
  _scenario?: string
}

export interface MockSearchResponse extends SearchResponse {
  _mockConfig?: MockConfig
}

export interface MockUser {
  id: string
  username: string
  email: string
  role: 'user' | 'admin'
  created_at: string
  is_active: boolean
  oidc_sub?: string
}

export interface MockSource {
  id: string
  name: string
  source_type: 'local_folder' | 'webdav' | 's3'
  path: string
  enabled: boolean
  user_id: string
  created_at: string
  updated_at: string
  last_sync_at?: string
  sync_status?: 'idle' | 'syncing' | 'error'
  webdav_config?: {
    url: string
    username: string
    password: string
    path: string
  }
  s3_config?: {
    bucket: string
    region: string
    access_key_id: string
    secret_access_key: string
    prefix?: string
  }
}

export interface MockLabel {
  id: string
  name: string
  color: string
  user_id: string
  created_at: string
  updated_at: string
  document_count?: number
}

export interface MockSyncProgress extends SyncProgressInfo {
  _mockConfig?: MockConfig
}

// WebSocket mock types
export interface MockWebSocketMessage {
  type: 'progress' | 'heartbeat' | 'error' | 'connection_confirmed' | 'connection_closing'
  data?: any
  _mockConfig?: MockConfig
}

// Error simulation types
export interface MockApiError {
  code: number
  message: string
  details?: any
  timestamp: string
}

// Response wrapper for consistent API responses
export interface MockApiResponse<T> {
  data: T
  status: number
  statusText: string
  headers: Record<string, string>
  _mockConfig?: MockConfig
}

// Factory options for generating mock data
export interface FactoryOptions {
  count?: number
  overrides?: Partial<any>
  scenario?: string
  seed?: number
}

// Mock state management
export interface MockState {
  documents: MockDocument[]
  users: MockUser[]
  sources: MockSource[]
  labels: MockLabel[]
  searchResults: Map<string, MockSearchResponse>
  syncProgress: Map<string, MockSyncProgress>
  queueStats: QueueStats
  scenarios: Record<string, any>
}

// Hook configuration for React testing utilities
export interface UseMockApiOptions {
  scenario?: string
  customHandlers?: any[]
  resetOnUnmount?: boolean
  defaultDelay?: number
}

// Window global extensions for E2E testing
declare global {
  interface Window {
    __MSW_ENABLED__?: boolean
    __MOCK_API_SCENARIO__?: string
    __MSW_READY__?: boolean
    __MOCK_NETWORK_DELAY__?: number
  }
}