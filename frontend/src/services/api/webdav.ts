import { sourceErrorService } from './sourceErrors'

// WebDAV Scan Failure Types
export interface WebDAVScanFailure {
  id: string
  directory_path: string
  failure_type: WebDAVScanFailureType
  failure_severity: WebDAVScanFailureSeverity
  failure_count: number
  consecutive_failures: number
  first_failure_at: string
  last_failure_at: string
  next_retry_at?: string
  error_message?: string
  http_status_code?: number
  user_excluded: boolean
  user_notes?: string
  resolved: boolean
  diagnostic_summary: WebDAVFailureDiagnostics
}

export type WebDAVScanFailureType =
  | 'timeout'
  | 'path_too_long'
  | 'permission_denied'
  | 'invalid_characters'
  | 'network_error'
  | 'server_error'
  | 'xml_parse_error'
  | 'too_many_items'
  | 'depth_limit'
  | 'size_limit'
  | 'unknown'

export type WebDAVScanFailureSeverity =
  | 'low'
  | 'medium'
  | 'high'
  | 'critical'

export interface WebDAVFailureDiagnostics {
  path_length?: number
  directory_depth?: number
  estimated_item_count?: number
  response_time_ms?: number
  response_size_mb?: number
  server_type?: string
  recommended_action: string
  can_retry: boolean
  user_action_required: boolean
}

export interface WebDAVScanFailureStats {
  active_failures: number
  resolved_failures: number
  excluded_directories: number
  critical_failures: number
  high_failures: number
  medium_failures: number
  low_failures: number
  ready_for_retry: number
}

export interface WebDAVScanFailuresResponse {
  failures: WebDAVScanFailure[]
  stats: WebDAVScanFailureStats
}

export interface RetryFailureRequest {
  notes?: string
}

export interface ExcludeFailureRequest {
  notes?: string
  permanent: boolean
}

// WebDAV Scan Failures Service (Backward Compatibility)
export const webdavService = {
  getScanFailures: async () => {
    // Redirect to generic service and transform response for backward compatibility
    const response = await sourceErrorService.getSourceFailuresByType('webdav')
    
    // Transform SourceScanFailure[] to WebDAVScanFailure[] format
    const transformedFailures: WebDAVScanFailure[] = response.data.failures.map(failure => ({
      id: failure.id,
      directory_path: failure.resource_path,
      failure_type: failure.error_type as WebDAVScanFailureType,
      failure_severity: failure.error_severity as WebDAVScanFailureSeverity,
      failure_count: failure.failure_count,
      consecutive_failures: failure.consecutive_failures,
      first_failure_at: failure.first_failure_at,
      last_failure_at: failure.last_failure_at,
      next_retry_at: failure.next_retry_at,
      error_message: failure.error_message,
      http_status_code: failure.status_code,
      user_excluded: failure.user_excluded,
      user_notes: failure.user_notes,
      resolved: failure.resolved,
      diagnostic_summary: failure.diagnostic_data as WebDAVFailureDiagnostics
    }))

    // Transform stats
    const transformedStats: WebDAVScanFailureStats = {
      active_failures: response.data.stats.active_failures,
      resolved_failures: response.data.stats.resolved_failures,
      excluded_directories: response.data.stats.excluded_resources,
      critical_failures: response.data.stats.critical_failures,
      high_failures: response.data.stats.high_failures,
      medium_failures: response.data.stats.medium_failures,
      low_failures: response.data.stats.low_failures,
      ready_for_retry: response.data.stats.ready_for_retry
    }

    return {
      ...response,
      data: {
        failures: transformedFailures,
        stats: transformedStats
      }
    }
  },

  getScanFailure: async (id: string) => {
    const response = await sourceErrorService.getSourceFailure(id)
    
    // Transform SourceScanFailure to WebDAVScanFailure format
    const transformedFailure: WebDAVScanFailure = {
      id: response.data.id,
      directory_path: response.data.resource_path,
      failure_type: response.data.error_type as WebDAVScanFailureType,
      failure_severity: response.data.error_severity as WebDAVScanFailureSeverity,
      failure_count: response.data.failure_count,
      consecutive_failures: response.data.consecutive_failures,
      first_failure_at: response.data.first_failure_at,
      last_failure_at: response.data.last_failure_at,
      next_retry_at: response.data.next_retry_at,
      error_message: response.data.error_message,
      http_status_code: response.data.status_code,
      user_excluded: response.data.user_excluded,
      user_notes: response.data.user_notes,
      resolved: response.data.resolved,
      diagnostic_summary: response.data.diagnostic_data as WebDAVFailureDiagnostics
    }

    return {
      ...response,
      data: transformedFailure
    }
  },

  retryFailure: async (id: string, request: RetryFailureRequest) => {
    const response = await sourceErrorService.retryFailure(id, request)
    
    return {
      ...response,
      data: {
        success: response.data.success,
        message: response.data.message,
        directory_path: response.data.resource_path
      }
    }
  },

  excludeFailure: async (id: string, request: ExcludeFailureRequest) => {
    const response = await sourceErrorService.excludeFailure(id, request)
    
    return {
      ...response,
      data: {
        success: response.data.success,
        message: response.data.message,
        directory_path: response.data.resource_path,
        permanent: response.data.permanent
      }
    }
  },

  getRetryCandidates: async () => {
    const response = await sourceErrorService.getRetryCandidates()
    
    return {
      ...response,
      data: {
        directories: response.data.resources,
        count: response.data.count
      }
    }
  }
}
