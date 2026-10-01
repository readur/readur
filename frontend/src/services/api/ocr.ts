import api from './client'
import type {
  AvailableLanguagesResponse,
  FailedDocument,
  RetryOcrRequest,
} from '../../types/generated'

export type {
  AvailableLanguagesResponse,
  BulkOcrRetryRequest,
  BulkOcrRetryResponse,
  LanguageInfo,
  OcrRetryDocumentInfo,
  OcrRetryFilter,
  RetryOcrRequest,
} from '../../types/generated'

// OCR Retry Types
export const ocrService = {
  getAvailableLanguages: () => {
    return api.get<AvailableLanguagesResponse>('/ocr/languages')
  },

  getHealthStatus: () => {
    return api.get('/ocr/health')
  },

  retryWithLanguage: (documentId: string, language?: string, languages?: string[]) => {
    const data: RetryOcrRequest = {}
    if (languages && languages.length > 0) {
      data.languages = languages
    } else if (language) {
      data.language = language
    }
    return api.post(`/documents/${documentId}/ocr/retry`, data)
  },

  /** GET /documents/failed with optional stage and reason filters. */
  listFailedDocuments: (params: FailedDocumentsParams = {}) => {
    const query: Record<string, string | number> = {
      limit: params.limit ?? 25,
      offset: params.offset ?? 0,
    }
    if (params.stage) query.stage = params.stage
    if (params.reason) query.reason = params.reason
    return api.get<FailedDocumentsResponse>('/documents/failed', { params: query })
  },

  /** The stored file of a failed document (GET /documents/failed/{id}), as a blob. */
  viewFailedDocument: (failedDocumentId: string) => {
    return api.get<Blob>(`/documents/failed/${failedDocumentId}`, { responseType: 'blob' })
  },
}

export interface FailedDocumentsParams {
  limit?: number
  offset?: number
  stage?: string
  reason?: string
}

/** One row of GET /documents/failed: the stored record plus fields the handler computes. */
export type FailedDocumentRow = Omit<FailedDocument, 'user_id' | 'file_hash' | 'original_path'> & {
  /** Human-readable failure category computed from the reason and message. */
  failure_category?: string
  /** Human-readable stage label ("OCR Processing", ...). */
  source?: string
  /** Only sent by /documents/failed/ocr; treat a missing value as retryable. */
  can_retry?: boolean
}

export interface FailedDocumentsResponse {
  documents: FailedDocumentRow[]
  pagination: { total: number; limit: number; offset: number; total_pages: number }
  statistics: {
    total_failed: number
    by_stage: Record<string, number>
    by_reason: Record<string, number>
  }
}
