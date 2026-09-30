import api from './client'
import type { ExcludeFailureRequest, RetryFailureRequest, WebDAVScanFailureType } from './webdav'
import type { SourceExcludeResponse, SourceRetryResponse } from './types'
import type { SourceType } from '../../types/generated'

export type { SourceType } from '../../types/generated'

// Generic Source Error Types (New System)
//
// TODO(ts-rs): SourceErrorType, SourceErrorSeverity, SourceScanFailure and
// SourceScanFailureStats are kept hand-written on purpose. The generated
// SourceErrorType/SourceErrorSeverity are PascalCase on the wire (the Rust enums
// have no serde rename; task-2b concern 1 - needs a backend/frontend decision),
// and SourceScanFailureResponse has a different shape (by_source_type, diagnostic_summary,
// no diagnostic_data/status_code). webdavService and the failure UI depend on the
// hand-written shape, so switching would change runtime behaviour.

export type SourceErrorType = WebDAVScanFailureType | 's3_access_denied' | 's3_bucket_not_found' | 's3_invalid_credentials' | 's3_network_error' | 'local_permission_denied' | 'local_path_not_found' | 'local_disk_full' | 'local_io_error'

export type SourceErrorSeverity = 'low' | 'medium' | 'high' | 'critical'

export interface SourceScanFailure {
  id: string
  user_id: string
  source_type: SourceType
  source_id?: string
  resource_path: string
  error_type: SourceErrorType
  error_severity: SourceErrorSeverity
  failure_count: number
  consecutive_failures: number
  first_failure_at: string
  last_failure_at: string
  next_retry_at?: string
  error_message: string
  error_code?: string
  status_code?: number
  user_excluded: boolean
  user_notes?: string
  resolved: boolean
  retry_strategy: 'exponential' | 'linear' | 'fixed'
  diagnostic_data: any // JSONB field with source-specific data
}

export interface SourceScanFailureStats {
  active_failures: number
  resolved_failures: number
  excluded_resources: number
  critical_failures: number
  high_failures: number
  medium_failures: number
  low_failures: number
  ready_for_retry: number
  failures_by_source_type: Record<SourceType, number>
  failures_by_error_type: Record<string, number>
}

export interface SourceScanFailuresResponse {
  failures: SourceScanFailure[]
  stats: SourceScanFailureStats
}


// Generic Source Error Service (New System)
export const sourceErrorService = {
  // Get all source failures
  getSourceFailures: () => {
    return api.get<SourceScanFailuresResponse>('/source/errors')
  },

  // Get specific failure by ID
  getSourceFailure: (id: string) => {
    return api.get<SourceScanFailure>(`/source/errors/${id}`)
  },

  // Get failures for specific source type
  getSourceFailuresByType: (sourceType: SourceType) => {
    return api.get<SourceScanFailuresResponse>(`/source/errors/type/${sourceType}`)
  },

  // Retry a specific failure
  retryFailure: (id: string, request: RetryFailureRequest) => {
    return api.post<SourceRetryResponse>(`/source/errors/${id}/retry`, request)
  },

  // Exclude a specific failure
  excludeFailure: (id: string, request: ExcludeFailureRequest) => {
    return api.post<SourceExcludeResponse>(`/source/errors/${id}/exclude`, request)
  },

  // Get retry candidates
  getRetryCandidates: () => {
    return api.get<{ resources: string[], count: number }>('/source/errors/retry/candidates')
  }
}
