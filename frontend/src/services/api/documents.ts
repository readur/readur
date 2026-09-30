import api from './client'
import type {
  DocumentOcrResponse,
  DocumentPaginationInfo,
  DocumentResponse,
  PaginatedDocumentsResponse,
} from '../../types/generated'
import { serializeFilterParams, type DocumentListParams } from './filterParams'
import type { BulkOcrRetryRequest, BulkOcrRetryResponse } from './ocr'
import type { SearchFacetsResponse, SearchRequest, SearchResponse } from './search'
import type {
  DocumentRetryHistoryResponse,
  OcrRetryRecommendationsResponse,
  OcrRetryStatsResponse,
} from './types'

export interface PaginatedResponse<T> {
  documents: T[]
  pagination: DocumentPaginationInfo
}

// TODO(ts-rs): the backend does not send these fields (ocr_error, ocr_failure_reason,
// ocr_retry_count, ocr_completed_at, ocr_word_count). They are kept as optional
// extras only so legacy readers (ActivityTab, OcrTextTab, DocumentDetailsHeader)
// keep compiling; they are always undefined at runtime. Drop them, and the
// intersections below, once those readers are rewritten.
interface LegacyDocumentFields {
  ocr_error?: string
  ocr_failure_reason?: string
  ocr_retry_count?: number
  ocr_completed_at?: string
}

interface LegacyOcrResponseFields {
  ocr_error?: string
  ocr_word_count?: number
  ocr_completed_at?: string
}

/** One row of GET /documents/failed/ocr: a real document whose OCR failed. */
export interface FailedOcrDocumentRow {
  id: string
  filename: string
  original_filename: string
  file_size: number
  mime_type: string
  created_at: string
  updated_at: string
  tags: string[]
  ocr_status: string | null
  ocr_error: string | null
  ocr_failure_reason: string | null
  ocr_completed_at: string | null
  retry_count: number
  last_attempt_at: string | null
  can_retry: boolean
  /** Human-readable category computed from the failure reason and error. */
  failure_category: string
}

export interface FailedOcrDocumentsResponse {
  documents: FailedOcrDocumentRow[]
  pagination: { total: number; limit: number; offset: number; has_more: boolean }
  statistics: { total_failed: number; failure_categories: unknown }
}

export type Document = DocumentResponse & LegacyDocumentFields
export type OcrResponse = DocumentOcrResponse & LegacyOcrResponseFields

export const documentService = {
  upload: (file: File, languages?: string[]) => {
    const formData = new FormData()
    formData.append('file', file)
    
    // Add multiple languages if provided
    if (languages && languages.length > 0) {
      languages.forEach((lang, index) => {
        formData.append(`ocr_languages[${index}]`, lang)
      })
    }
    
    return api.post('/documents', formData, {
      headers: {
        'Content-Type': 'multipart/form-data',
      },
    })
  },

  list: (limit = 50, offset = 0) => {
    return api.get<Document[]>('/documents', {
      params: { limit, offset },
    })
  },

  listWithPagination: (limit = 20, offset = 0, ocrStatus?: string) => {
    const params: any = { limit, offset };
    if (ocrStatus) {
      params.ocr_status = ocrStatus;
    }
    return api.get<{documents: Document[], pagination: {total: number, limit: number, offset: number, has_more: boolean}}>('/documents', {
      params,
    })
  },

  /**
   * Server-side sorted and filtered list (GET /documents). List filters
   * (`label_ids`, `source_ids`, `mime_types`, `source_types`, `tags`) accept
   * arrays and are sent comma-separated; `created_from` / `created_to` are
   * sent as ISO strings. Unset values are omitted.
   */
  listFiltered: (params: DocumentListParams = {}) => {
    return api.get<PaginatedDocumentsResponse>('/documents', {
      params: serializeFilterParams(params),
    })
  },

  getById: (id: string) => {
    return api.get<Document>(`/documents/${id}`)
  },

  download: (id: string) => {
    return api.get(`/documents/${id}/download`, {
      responseType: 'blob',
    })
  },

  downloadFile: async (id: string, filename?: string) => {
    try {
      const response = await api.get(`/documents/${id}/download`, {
        responseType: 'blob',
      });
      
      // Create blob URL and trigger download
      const blob = new Blob([response.data], { type: response.headers['content-type'] });
      const url = window.URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = filename || `document-${id}`;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      window.URL.revokeObjectURL(url);
    } catch (error) {
      console.error('Download failed:', error);
      throw error;
    }
  },

  getOcrText: (id: string) => {
    return api.get<OcrResponse>(`/documents/${id}/ocr`)
  },

  view: (id: string) => {
    return api.get(`/documents/${id}/view`, {
      responseType: 'blob',
    })
  },

  getThumbnail: (id: string) => {
    return api.get(`/documents/${id}/thumbnail`, {
      responseType: 'blob',
    })
  },

  getProcessedImage: (id: string) => {
    return api.get(`/documents/${id}/processed/image`, {
      responseType: 'blob',
    })
  },

  retryOcr: (id: string) => {
    return api.post(`/documents/${id}/ocr/retry`)
  },

  // Advanced OCR retry functionality
  bulkRetryOcr: (request: BulkOcrRetryRequest) => {
    return api.post<BulkOcrRetryResponse>('/documents/ocr/retry/bulk', request)
  },

  getRetryStats: () => {
    return api.get<OcrRetryStatsResponse>('/documents/ocr/retry/stats')
  },

  getRetryRecommendations: () => {
    return api.get<OcrRetryRecommendationsResponse>('/documents/ocr/retry/recommendations')
  },

  getDocumentRetryHistory: (id: string) => {
    return api.get<DocumentRetryHistoryResponse>(`/documents/${id}/ocr/retry/history`)
  },

  /**
   * Documents whose OCR failed (GET /documents/failed/ocr). Rows carry the real
   * `documents.id`, so every per-document action (retry, delete, download,
   * retry history) can use them. `/documents/failed` returns failed-import
   * records instead, whose ids are not document ids.
   */
  getFailedOcrDocuments: (limit = 50, offset = 0) => {
    return api.get<FailedOcrDocumentsResponse>(`/documents/failed/ocr`, {
      params: { limit, offset },
    })
  },

  getDuplicates: (limit = 25, offset = 0) => {
    return api.get(`/documents/duplicates`, {
      params: { limit, offset },
    })
  },

  search: (searchRequest: SearchRequest) => {
    return api.get<SearchResponse>('/search', {
      params: searchRequest,
    })
  },

  enhancedSearch: (searchRequest: SearchRequest) => {
    return api.get<SearchResponse>('/search/enhanced', {
      params: {
        ...searchRequest,
        include_snippets: searchRequest.include_snippets ?? true,
        snippet_length: searchRequest.snippet_length ?? 200,
        search_mode: searchRequest.search_mode ?? 'simple',
      },
    })
  },

  getFacets: () => {
    return api.get<SearchFacetsResponse>('/search/facets')
  },

  delete: (id: string) => {
    return api.delete(`/documents/${id}`)
  },

  bulkDelete: (documentIds: string[]) => {
    return api.post('/documents/bulk/delete', {
      document_ids: documentIds
    })
  },

  deleteLowConfidence: (maxConfidence: number, previewOnly: boolean = false) => {
    return api.delete('/documents/cleanup/low/confidence', {
      data: {
        max_confidence: maxConfidence,
        preview_only: previewOnly
      }
    })
  },
  deleteFailedOcr: (previewOnly: boolean = false) => {
    return api.delete('/documents/cleanup/failed/ocr', {
      data: {
        preview_only: previewOnly
      }
    })
  },

  getFailedDocuments: (limit = 25, offset = 0, stage?: string, reason?: string) => {
    const params: any = { limit, offset };
    if (stage) params.stage = stage;
    if (reason) params.reason = reason;
    return api.get('/documents/failed', { params })
  },
}
