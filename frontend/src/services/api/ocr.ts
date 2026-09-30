import api from './client'
import type {
  AvailableLanguagesResponse,
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
}
