import api from './client'

// OCR Retry Types
export interface OcrRetryFilter {
  mime_types?: string[]
  file_extensions?: string[]
  failure_reasons?: string[]
  min_file_size?: number
  max_file_size?: number
  created_after?: string
  created_before?: string
  tags?: string[]
  limit?: number
}

export interface BulkOcrRetryRequest {
  mode: 'all' | 'specific' | 'filter'
  document_ids?: string[]
  filter?: OcrRetryFilter
  priority_override?: number
  preview_only?: boolean
}

export interface OcrRetryDocumentInfo {
  id: string
  filename: string
  file_size: number
  mime_type: string
  ocr_failure_reason?: string
  priority: number
  queue_id?: string
}

export interface BulkOcrRetryResponse {
  success: boolean
  message: string
  queued_count: number
  matched_count: number
  documents: OcrRetryDocumentInfo[]
  estimated_total_time_minutes: number
}

export interface LanguageInfo {
  code: string
  name: string
  installed: boolean
}

export interface AvailableLanguagesResponse {
  available_languages: LanguageInfo[]
  current_user_language: string
}

export interface RetryOcrRequest {
  language?: string
  languages?: string[]
}

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
}
