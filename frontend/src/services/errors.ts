import axios, { AxiosError } from 'axios'

// Error Response Interfaces
export interface ApiErrorResponse {
  error: string
  code: string
  status: number
}

export interface AxiosErrorWithCode extends AxiosError {
  response?: {
    data: ApiErrorResponse
    status: number
    statusText: string
    headers: any
  }
}

// Error Code Constants
export const ErrorCodes = {
  // User Errors
  USER_NOT_FOUND: 'USER_NOT_FOUND',
  USER_NOT_FOUND_BY_ID: 'USER_NOT_FOUND_BY_ID',
  USER_DUPLICATE_USERNAME: 'USER_DUPLICATE_USERNAME',
  USER_DUPLICATE_EMAIL: 'USER_DUPLICATE_EMAIL',
  USER_INVALID_ROLE: 'USER_INVALID_ROLE',
  USER_PERMISSION_DENIED: 'USER_PERMISSION_DENIED',
  USER_INVALID_CREDENTIALS: 'USER_INVALID_CREDENTIALS',
  USER_ACCOUNT_DISABLED: 'USER_ACCOUNT_DISABLED',
  USER_INVALID_PASSWORD: 'USER_INVALID_PASSWORD',
  USER_INVALID_USERNAME: 'USER_INVALID_USERNAME',
  USER_INVALID_EMAIL: 'USER_INVALID_EMAIL',
  USER_DELETE_RESTRICTED: 'USER_DELETE_RESTRICTED',
  USER_OIDC_AUTH_FAILED: 'USER_OIDC_AUTH_FAILED',
  USER_AUTH_PROVIDER_NOT_CONFIGURED: 'USER_AUTH_PROVIDER_NOT_CONFIGURED',
  USER_TOKEN_EXPIRED: 'USER_TOKEN_EXPIRED',
  USER_INVALID_TOKEN: 'USER_INVALID_TOKEN',
  USER_SESSION_EXPIRED: 'USER_SESSION_EXPIRED',
  USER_INTERNAL_SERVER_ERROR: 'USER_INTERNAL_SERVER_ERROR',

  // Source Errors
  SOURCE_NOT_FOUND: 'SOURCE_NOT_FOUND',
  SOURCE_DUPLICATE_NAME: 'SOURCE_DUPLICATE_NAME',
  SOURCE_INVALID_NAME: 'SOURCE_INVALID_NAME',
  SOURCE_INVALID_PATH: 'SOURCE_INVALID_PATH',
  SOURCE_CONNECTION_FAILED: 'SOURCE_CONNECTION_FAILED',
  SOURCE_AUTH_FAILED: 'SOURCE_AUTH_FAILED',
  SOURCE_PERMISSION_DENIED: 'SOURCE_PERMISSION_DENIED',
  SOURCE_QUOTA_EXCEEDED: 'SOURCE_QUOTA_EXCEEDED',
  SOURCE_RATE_LIMIT_EXCEEDED: 'SOURCE_RATE_LIMIT_EXCEEDED',
  SOURCE_CONFIG_INVALID: 'SOURCE_CONFIG_INVALID',
  SOURCE_SYNC_IN_PROGRESS: 'SOURCE_SYNC_IN_PROGRESS',
  SOURCE_UNSUPPORTED_OPERATION: 'SOURCE_UNSUPPORTED_OPERATION',
  SOURCE_NETWORK_TIMEOUT: 'SOURCE_NETWORK_TIMEOUT',

  // Label Errors
  LABEL_NOT_FOUND: 'LABEL_NOT_FOUND',
  LABEL_DUPLICATE_NAME: 'LABEL_DUPLICATE_NAME',
  LABEL_INVALID_NAME: 'LABEL_INVALID_NAME',
  LABEL_INVALID_COLOR: 'LABEL_INVALID_COLOR',
  LABEL_SYSTEM_MODIFICATION: 'LABEL_SYSTEM_MODIFICATION',
  LABEL_IN_USE: 'LABEL_IN_USE',
  LABEL_MAX_LABELS_REACHED: 'LABEL_MAX_LABELS_REACHED',

  // Settings Errors
  SETTINGS_INVALID_LANGUAGE: 'SETTINGS_INVALID_LANGUAGE',
  SETTINGS_VALUE_OUT_OF_RANGE: 'SETTINGS_VALUE_OUT_OF_RANGE',
  SETTINGS_INVALID_VALUE: 'SETTINGS_INVALID_VALUE',
  SETTINGS_INVALID_OCR_CONFIG: 'SETTINGS_INVALID_OCR_CONFIG',
  SETTINGS_CONFLICTING_SETTINGS: 'SETTINGS_CONFLICTING_SETTINGS',

  // Search Errors
  SEARCH_QUERY_TOO_SHORT: 'SEARCH_QUERY_TOO_SHORT',
  SEARCH_TOO_MANY_RESULTS: 'SEARCH_TOO_MANY_RESULTS',
  SEARCH_INDEX_UNAVAILABLE: 'SEARCH_INDEX_UNAVAILABLE',
  SEARCH_INVALID_PAGINATION: 'SEARCH_INVALID_PAGINATION',
  SEARCH_NO_RESULTS: 'SEARCH_NO_RESULTS',

  // Document Errors
  DOCUMENT_NOT_FOUND: 'DOCUMENT_NOT_FOUND',
  DOCUMENT_UPLOAD_FAILED: 'DOCUMENT_UPLOAD_FAILED',
  DOCUMENT_INVALID_FORMAT: 'DOCUMENT_INVALID_FORMAT',
  DOCUMENT_TOO_LARGE: 'DOCUMENT_TOO_LARGE',
  DOCUMENT_OCR_FAILED: 'DOCUMENT_OCR_FAILED',
} as const

export type ErrorCode = typeof ErrorCodes[keyof typeof ErrorCodes]

// Error Helper Functions
export const ErrorHelper = {
  /**
   * Extract error information from an axios error
   */
  getErrorInfo: (error: unknown): { message: string; code?: string; status?: number } => {
    if (axios.isAxiosError(error)) {
      const axiosError = error as AxiosErrorWithCode
      
      // Check if it's our new structured error format
      if (axiosError.response?.data?.error && axiosError.response?.data?.code) {
        return {
          message: axiosError.response.data.error,
          code: axiosError.response.data.code,
          status: axiosError.response.data.status || axiosError.response.status,
        }
      }
      
      // Fallback to legacy error handling
      if (axiosError.response?.data?.message) {
        return {
          message: axiosError.response.data.message,
          status: axiosError.response.status,
        }
      }
      
      // Default axios error handling
      return {
        message: axiosError.message || 'An error occurred',
        status: axiosError.response?.status,
      }
    }
    
    // Handle non-axios errors
    if (error instanceof Error) {
      return { message: error.message }
    }
    
    return { message: 'An unknown error occurred' }
  },

  /**
   * Check if error is a specific error code
   */
  isErrorCode: (error: unknown, code: ErrorCode): boolean => {
    const errorInfo = ErrorHelper.getErrorInfo(error)
    return errorInfo.code === code
  },

  /**
   * Get user-friendly error message with fallback
   */
  getUserMessage: (error: unknown, fallback?: string): string => {
    const errorInfo = ErrorHelper.getErrorInfo(error)
    return errorInfo.message || fallback || 'An error occurred'
  },

  /**
   * Get suggested action based on error code
   */
  getSuggestedAction: (error: unknown): string | null => {
    const errorInfo = ErrorHelper.getErrorInfo(error)
    
    switch (errorInfo.code) {
      case ErrorCodes.USER_DUPLICATE_USERNAME:
        return 'Please choose a different username'
      case ErrorCodes.USER_DUPLICATE_EMAIL:
        return 'Please use a different email address'
      case ErrorCodes.USER_INVALID_PASSWORD:
        return 'Password must be at least 8 characters with uppercase, lowercase, and numbers'
      case ErrorCodes.USER_INVALID_CREDENTIALS:
        return 'Please check your username and password'
      case ErrorCodes.USER_SESSION_EXPIRED:
      case ErrorCodes.USER_TOKEN_EXPIRED:
        return 'Please login again'
      case ErrorCodes.USER_ACCOUNT_DISABLED:
        return 'Please contact an administrator'
      case ErrorCodes.SOURCE_CONNECTION_FAILED:
        return 'Check your network connection and server settings'
      case ErrorCodes.SOURCE_AUTH_FAILED:
        return 'Verify your credentials and try again'
      case ErrorCodes.SOURCE_CONFIG_INVALID:
        return 'Check your source configuration settings'
      case ErrorCodes.LABEL_DUPLICATE_NAME:
        return 'Please choose a different label name'
      case ErrorCodes.LABEL_INVALID_COLOR:
        return 'Use a valid hex color format like #0969da'
      case ErrorCodes.SEARCH_QUERY_TOO_SHORT:
        return 'Please enter at least 2 characters'
      case ErrorCodes.DOCUMENT_TOO_LARGE:
        return 'Please select a smaller file'
      case ErrorCodes.SETTINGS_INVALID_LANGUAGE:
        return 'Please select a valid language from the available options'
      case ErrorCodes.SETTINGS_VALUE_OUT_OF_RANGE:
        return 'Please enter a value within the allowed range'
      case ErrorCodes.SETTINGS_INVALID_OCR_CONFIG:
        return 'Please check your OCR configuration settings'
      case ErrorCodes.SETTINGS_CONFLICTING_SETTINGS:
        return 'Please resolve conflicting settings before saving'
      default:
        return null
    }
  },

  /**
   * Check if error should show retry option
   */
  shouldShowRetry: (error: unknown): boolean => {
    const errorInfo = ErrorHelper.getErrorInfo(error)
    
    const retryableCodes = [
      ErrorCodes.SOURCE_CONNECTION_FAILED,
      ErrorCodes.SOURCE_NETWORK_TIMEOUT,
      ErrorCodes.SOURCE_RATE_LIMIT_EXCEEDED,
      ErrorCodes.SEARCH_INDEX_UNAVAILABLE,
      ErrorCodes.DOCUMENT_UPLOAD_FAILED,
      ErrorCodes.DOCUMENT_OCR_FAILED,
    ]
    
    return retryableCodes.includes(errorInfo.code as ErrorCode) || 
           (errorInfo.status && errorInfo.status >= 500)
  },

  /**
   * Get error category for styling/icons
   */
  getErrorCategory: (error: unknown): 'auth' | 'validation' | 'network' | 'server' | 'unknown' => {
    const errorInfo = ErrorHelper.getErrorInfo(error)
    
    if (errorInfo.code?.startsWith('USER_')) {
      if (['USER_INVALID_CREDENTIALS', 'USER_TOKEN_EXPIRED', 'USER_SESSION_EXPIRED'].includes(errorInfo.code)) {
        return 'auth'
      }
      if (['USER_INVALID_PASSWORD', 'USER_INVALID_EMAIL', 'USER_INVALID_USERNAME'].includes(errorInfo.code)) {
        return 'validation'
      }
    }
    
    if (errorInfo.code?.includes('CONNECTION_FAILED') || errorInfo.code?.includes('NETWORK_TIMEOUT')) {
      return 'network'
    }
    
    if (errorInfo.code?.includes('INVALID') || errorInfo.code?.includes('OUT_OF_RANGE')) {
      return 'validation'
    }
    
    if (errorInfo.status && errorInfo.status >= 500) {
      return 'server'
    }
    
    if (errorInfo.status === 401 || errorInfo.status === 403) {
      return 'auth'
    }
    
    if (errorInfo.status === 400 || errorInfo.status === 422) {
      return 'validation'
    }
    
    return 'unknown'
  },

  /**
   * Get appropriate error icon based on category
   */
  getErrorIcon: (error: unknown): string => {
    const category = ErrorHelper.getErrorCategory(error)
    
    switch (category) {
      case 'auth':
        return '🔒'
      case 'validation':
        return '⚠️'
      case 'network':
        return '🌐'
      case 'server':
        return '🔧'
      default:
        return '❌'
    }
  },

  /**
   * Format error for display in UI components
   */
  formatErrorForDisplay: (error: unknown, includeActions?: boolean) => {
    const errorInfo = ErrorHelper.getErrorInfo(error)
    const suggestedAction = ErrorHelper.getSuggestedAction(error)
    const shouldRetry = ErrorHelper.shouldShowRetry(error)
    const category = ErrorHelper.getErrorCategory(error)
    const icon = ErrorHelper.getErrorIcon(error)

    return {
      message: errorInfo.message,
      code: errorInfo.code,
      status: errorInfo.status,
      suggestedAction: includeActions ? suggestedAction : null,
      shouldShowRetry: includeActions ? shouldRetry : false,
      category,
      icon,
      severity: category === 'validation' ? 'warning' : 
                category === 'auth' ? 'info' : 'error'
    }
  },

  /**
   * Handle specific error codes with custom logic
   */
  handleSpecificError: (error: unknown, onRetry?: () => void, onLogin?: () => void) => {
    const errorInfo = ErrorHelper.getErrorInfo(error)
    
    switch (errorInfo.code) {
      case ErrorCodes.USER_SESSION_EXPIRED:
      case ErrorCodes.USER_TOKEN_EXPIRED:
        if (onLogin) {
          onLogin()
          return true // Handled
        }
        break
      
      case ErrorCodes.SOURCE_CONNECTION_FAILED:
      case ErrorCodes.SOURCE_NETWORK_TIMEOUT:
        if (onRetry && ErrorHelper.shouldShowRetry(error)) {
          // Could automatically retry after a delay
          setTimeout(onRetry, 2000)
          return true // Handled
        }
        break
    }
    
    return false // Not handled
  }
}