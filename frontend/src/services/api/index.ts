// Barrel: every name the former services/api.ts exported, so
// `import ... from '…/services/api'` keeps working unchanged.
import api from './client'

export { api }
export default api

// Re-export error handling utilities for convenience
export { ErrorHelper, ErrorCodes } from '../errors'
export type { ApiErrorResponse, AxiosErrorWithCode, ErrorCode } from '../errors'

export * from './documents'
export * from './search'
export * from './filterParams'
export * from './ocr'
export * from './queue'
export * from './syncProgress'
export * from './sources'
export * from './sourceErrors'
export * from './webdav'
export * from './userWatch'
export * from './sharedLinks'
export * from './comments'
export * from './apiKeys'
export * from './labels'
export * from './types'
