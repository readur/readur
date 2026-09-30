import api from './client'
import type { BulkOcrRetryRequest, BulkOcrRetryResponse } from './ocr'
import type { SearchFacetsResponse, SearchRequest, SearchResponse } from './search'
import type {
  DocumentRetryHistoryResponse,
  OcrRetryRecommendationsResponse,
  OcrRetryStatsResponse,
} from './types'

export interface Document {
  id: string
  filename: string
  original_filename: string
  file_path: string
  file_size: number
  mime_type: string
  tags: string[]
  created_at: string
  updated_at: string
  user_id: string
  username?: string
  file_hash?: string
  original_created_at?: string
  original_modified_at?: string
  source_path?: string
  source_type?: string
  source_id?: string
  file_permissions?: number
  file_owner?: string
  file_group?: string
  source_metadata?: Record<string, any>
  has_ocr_text: boolean
  ocr_confidence?: number
  ocr_word_count?: number
  ocr_processing_time_ms?: number
  ocr_status?: string
  ocr_progress_current?: number
  ocr_progress_total?: number
  ocr_error?: string
  ocr_failure_reason?: string
  ocr_retry_count?: number
  ocr_completed_at?: string
}

export interface PaginatedResponse<T> {
  documents: T[]
  pagination: {
    total: number
    limit: number
    offset: number
    has_more: boolean
  }
}

export interface OcrResponse {
  document_id: string
  filename: string
  has_ocr_text: boolean
  ocr_text?: string
  ocr_confidence?: number
  ocr_word_count?: number
  ocr_processing_time_ms?: number
  ocr_status?: string
  ocr_error?: string
  ocr_failure_reason?: string
  ocr_retry_count?: number
  ocr_completed_at?: string
}

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

  getFailedOcrDocuments: (limit = 50, offset = 0) => {
    return api.get(`/documents/failed`, {
      params: { stage: 'ocr', limit, offset },
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
