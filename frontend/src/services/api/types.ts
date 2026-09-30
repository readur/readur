// Frontend-only types: shapes the backend returns as untyped JSON, so there is
// no generated (ts-rs) counterpart.
import type { OcrRetryFilter } from './ocr'
import type { SourceType } from './sourceErrors'

export interface OcrRetryStatsResponse {
  failure_reasons: Array<{
    reason: string
    count: number
    avg_file_size_mb: number
    first_occurrence: string
    last_occurrence: string
  }>
  file_types: Array<{
    mime_type: string
    count: number
    avg_file_size_mb: number
  }>
  total_failed: number
}

export interface OcrRetryRecommendation {
  reason: string
  title: string
  description: string
  estimated_success_rate: number
  document_count: number
  filter: OcrRetryFilter
}

export interface OcrRetryRecommendationsResponse {
  recommendations: OcrRetryRecommendation[]
  total_recommendations: number
}

export interface DocumentRetryHistoryItem {
  id: string
  retry_reason: string
  previous_status?: string
  previous_failure_reason?: string
  previous_error?: string
  priority: number
  queue_id?: string
  created_at: string
}

export interface DocumentRetryHistoryResponse {
  document_id: string
  retry_history: DocumentRetryHistoryItem[]
  total_retries: number
}

export interface QueueStats {
  pending_count: number
  processing_count: number
  failed_count: number
  completed_today: number
  avg_wait_time_minutes?: number
  oldest_pending_minutes?: number
}

export interface OcrStatusResponse {
  is_paused: boolean
  status: 'paused' | 'running'
}

export interface OcrActionResponse {
  status: 'paused' | 'resumed'
  message: string
}

export interface WebSocketMessage {
  type: 'progress' | 'heartbeat' | 'error' | 'connection_confirmed' | 'connection_closing';
  data?: any;
}

export interface RetryResponse {
  success: boolean
  message: string
  directory_path: string
}

export interface ExcludeResponse {
  success: boolean
  message: string
  directory_path: string
  permanent: boolean
}

export interface SourceRetryResponse {
  success: boolean
  message: string
  resource_path: string
  source_type: SourceType
}

export interface SourceExcludeResponse {
  success: boolean
  message: string
  resource_path: string
  source_type: SourceType
  permanent: boolean
}
