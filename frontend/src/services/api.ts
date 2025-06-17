import axios from 'axios'

const api = axios.create({
  baseURL: '/api',
  headers: {
    'Content-Type': 'application/json',
  },
})

export { api }
export default api

export interface Document {
  id: string
  filename: string
  original_filename: string
  file_size: number
  mime_type: string
  tags: string[]
  created_at: string
  has_ocr_text: boolean
  ocr_confidence?: number
  ocr_word_count?: number
  ocr_processing_time_ms?: number
  ocr_status?: string
}

export interface SearchRequest {
  query: string
  tags?: string[]
  mime_types?: string[]
  limit?: number
  offset?: number
  include_snippets?: boolean
  snippet_length?: number
  search_mode?: 'simple' | 'phrase' | 'fuzzy' | 'boolean'
}

export interface HighlightRange {
  start: number
  end: number
}

export interface SearchSnippet {
  text: string
  start_offset: number
  end_offset: number
  highlight_ranges: HighlightRange[]
}

export interface EnhancedDocument {
  id: string
  filename: string
  original_filename: string
  file_size: number
  mime_type: string
  tags: string[]
  created_at: string
  has_ocr_text: boolean
  ocr_confidence?: number
  ocr_word_count?: number
  ocr_processing_time_ms?: number
  ocr_status?: string
  search_rank?: number
  snippets: SearchSnippet[]
}

export interface SearchResponse {
  documents: EnhancedDocument[]
  total: number
  query_time_ms: number
  suggestions: string[]
}

export interface FacetItem {
  value: string
  count: number
}

export interface SearchFacetsResponse {
  mime_types: FacetItem[]
  tags: FacetItem[]
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

export interface QueueStats {
  pending_count: number
  processing_count: number
  failed_count: number
  completed_today: number
  avg_wait_time_minutes?: number
  oldest_pending_minutes?: number
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
  ocr_completed_at?: string
}

export const documentService = {
  upload: (file: File) => {
    const formData = new FormData()
    formData.append('file', file)
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
    return api.get(`/documents/${id}/processed-image`, {
      responseType: 'blob',
    })
  },

  retryOcr: (id: string) => {
    return api.post(`/documents/${id}/retry-ocr`)
  },

  getFailedOcrDocuments: (limit = 50, offset = 0) => {
    return api.get(`/documents/failed-ocr`, {
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
}

export interface OcrStatusResponse {
  is_paused: boolean
  status: 'paused' | 'running'
}

export interface OcrActionResponse {
  status: 'paused' | 'resumed'
  message: string
}

export const queueService = {
  getStats: () => {
    return api.get<QueueStats>('/queue/stats')
  },

  requeueFailed: () => {
    return api.post('/queue/requeue-failed')
  },

  getOcrStatus: () => {
    return api.get<OcrStatusResponse>('/queue/status')
  },

  pauseOcr: () => {
    return api.post<OcrActionResponse>('/queue/pause')
  },

  resumeOcr: () => {
    return api.post<OcrActionResponse>('/queue/resume')
  },
}