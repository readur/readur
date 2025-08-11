import axios from 'axios'

const api = axios.create({
  baseURL: '/api',
  headers: {
    'Content-Type': 'application/json',
  },
})

export { api }
export default api

// Re-export error handling utilities for convenience
export { ErrorHelper, ErrorCodes } from './errors'
export type { ApiErrorResponse, AxiosErrorWithCode, ErrorCode } from './errors'

export interface SyncProgressInfo {
  source_id: string
  phase: string
  phase_description: string
  elapsed_time_secs: number
  directories_found: number
  directories_processed: number
  files_found: number
  files_processed: number
  bytes_processed: number
  processing_rate_files_per_sec: number
  files_progress_percent: number
  estimated_time_remaining_secs?: number
  current_directory: string
  current_file?: string
  errors: number
  warnings: number
  is_active: boolean
}

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

export interface OcrRetryStatsResponse {
  failure_reasons: Array<{
    reason: string
    count: number
    avg_file_size_mb: number
    first_occurrence: string
    last_occurrence: string
  }>
  file_types: Array<{
    mime_type: string
    count: number
    avg_file_size_mb: number
  }>
  total_failed: number
}

export interface OcrRetryRecommendation {
  reason: string
  title: string
  description: string
  estimated_success_rate: number
  document_count: number
  filter: OcrRetryFilter
}

export interface OcrRetryRecommendationsResponse {
  recommendations: OcrRetryRecommendation[]
  total_recommendations: number
}

export interface DocumentRetryHistoryItem {
  id: string
  retry_reason: string
  previous_status?: string
  previous_failure_reason?: string
  previous_error?: string
  priority: number
  queue_id?: string
  created_at: string
}

export interface DocumentRetryHistoryResponse {
  document_id: string
  retry_history: DocumentRetryHistoryItem[]
  total_retries: number
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
    return api.get(`/documents/${id}/processed-image`, {
      responseType: 'blob',
    })
  },

  retryOcr: (id: string) => {
    return api.post(`/documents/${id}/ocr/retry`)
  },

  // Advanced OCR retry functionality
  bulkRetryOcr: (request: BulkOcrRetryRequest) => {
    return api.post<BulkOcrRetryResponse>('/documents/ocr/bulk-retry', request)
  },

  getRetryStats: () => {
    return api.get<OcrRetryStatsResponse>('/documents/ocr/retry-stats')
  },

  getRetryRecommendations: () => {
    return api.get<OcrRetryRecommendationsResponse>('/documents/ocr/retry-recommendations')
  },

  getDocumentRetryHistory: (id: string) => {
    return api.get<DocumentRetryHistoryResponse>(`/documents/${id}/ocr/retry-history`)
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
    return api.post('/documents/delete-low-confidence', {
      max_confidence: maxConfidence,
      preview_only: previewOnly
    })
  },
  deleteFailedOcr: (previewOnly: boolean = false) => {
    return api.post('/documents/delete-failed-ocr', {
      preview_only: previewOnly
    })
  },

  getFailedDocuments: (limit = 25, offset = 0, stage?: string, reason?: string) => {
    const params: any = { limit, offset };
    if (stage) params.stage = stage;
    if (reason) params.reason = reason;
    return api.get('/documents/failed', { params })
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

export const queueService = {
  getStats: () => {
    return api.get<QueueStats>('/queue/stats')
  },

  requeueFailed: () => {
    return api.post('/queue/requeue/failed')
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

export interface WebSocketMessage {
  type: 'progress' | 'heartbeat' | 'error' | 'connection_confirmed' | 'connection_closing';
  data?: any;
}

export class SyncProgressWebSocket {
  private ws: WebSocket | null = null;
  private sourceId: string;
  private url: string;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;
  private isManuallyClosing = false;
  private listeners: { [key: string]: ((data: any) => void)[] } = {};

  constructor(sourceId: string) {
    this.sourceId = sourceId;
    this.url = this.buildWebSocketUrl(sourceId);
  }

  private buildWebSocketUrl(sourceId: string): string {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    
    // In development, frontend (5173) and backend (8000) run on different ports
    // WebSocket connections can't use Vite proxy, so we need to connect directly to backend
    const isDevelopment = import.meta.env.DEV || window.location.port === '5173';
    const host = isDevelopment ? 'localhost:8000' : window.location.host;
    
    return `${protocol}//${host}/api/sources/${sourceId}/sync/progress/ws`;
  }
  
  private getAuthProtocol(): string | undefined {
    const token = localStorage.getItem('token');
    return token ? `bearer.${token}` : undefined;
  }

  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        // Create WebSocket connection with secure authentication via protocol header
        const authProtocol = this.getAuthProtocol();
        this.ws = authProtocol 
          ? new WebSocket(this.url, [authProtocol])
          : new WebSocket(this.url);

        this.ws.onopen = () => {
          console.log(`WebSocket connected to sync progress for source: ${this.sourceId}`);
          this.reconnectAttempts = 0;
          this.emit('connectionStatus', 'connected');
          resolve();
        };

        this.ws.onmessage = (event) => {
          try {
            const message: WebSocketMessage = JSON.parse(event.data);
            
            switch (message.type) {
              case 'progress':
                this.emit('progress', message.data);
                break;
              case 'heartbeat':
                this.emit('heartbeat', message.data);
                break;
              case 'error':
                this.emit('error', message.data);
                console.error('WebSocket error from server:', message.data);
                break;
              case 'connection_confirmed':
                this.emit('connectionConfirmed', message.data);
                break;
              case 'connection_closing':
                this.emit('connectionClosing', message.data);
                console.log('Server is closing connection:', message.data);
                break;
              default:
                console.warn('Unknown WebSocket message type:', message.type);
            }
          } catch (error) {
            console.error('Failed to parse WebSocket message:', error);
            this.emit('error', { error: 'Failed to parse message' });
          }
        };

        this.ws.onclose = (event) => {
          console.log(`WebSocket closed for source ${this.sourceId}:`, event.code, event.reason);
          this.emit('connectionStatus', 'disconnected');
          
          if (!this.isManuallyClosing && this.shouldReconnect(event.code)) {
            this.scheduleReconnect();
          }
        };

        this.ws.onerror = (error) => {
          console.error('WebSocket error:', error);
          this.emit('connectionStatus', 'error');
          this.emit('error', { error: 'WebSocket connection error' });
          reject(error);
        };

      } catch (error) {
        console.error('Failed to create WebSocket:', error);
        this.emit('connectionStatus', 'error');
        reject(error);
      }
    });
  }

  private shouldReconnect(code: number): boolean {
    // Don't reconnect on normal closure, authentication failure, or when max attempts reached
    // WebSocket close codes: 1000 = normal, 1001 = going away, 1003 = unsupported data, 1008 = policy violation (auth)
    const noReconnectCodes = [1000, 1001, 1003, 1008];
    return !noReconnectCodes.includes(code) && this.reconnectAttempts < this.maxReconnectAttempts;
  }

  private scheduleReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error('Max reconnection attempts reached for WebSocket');
      this.emit('connectionStatus', 'failed');
      return;
    }

    this.reconnectAttempts++;
    const delay = Math.min(this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1), 30000);
    
    console.log(`Attempting to reconnect WebSocket in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`);
    this.emit('connectionStatus', 'reconnecting');

    setTimeout(() => {
      if (!this.isManuallyClosing) {
        this.connect().catch(error => {
          console.error('Reconnection failed:', error);
        });
      }
    }, delay);
  }

  addEventListener(eventType: string, callback: (data: any) => void): void {
    if (!this.listeners[eventType]) {
      this.listeners[eventType] = [];
    }
    this.listeners[eventType].push(callback);
  }

  removeEventListener(eventType: string, callback: (data: any) => void): void {
    if (this.listeners[eventType]) {
      this.listeners[eventType] = this.listeners[eventType].filter(cb => cb !== callback);
    }
  }

  private emit(eventType: string, data: any): void {
    if (this.listeners[eventType]) {
      this.listeners[eventType].forEach(callback => callback(data));
    }
  }

  close(): void {
    this.isManuallyClosing = true;
    if (this.ws) {
      this.ws.close(1000, 'Client requested closure');
      this.ws = null;
    }
    this.listeners = {};
  }

  getReadyState(): number {
    return this.ws?.readyState ?? WebSocket.CLOSED;
  }

  isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }
}

// User Watch Directory Types
export interface UserWatchDirectoryResponse {
  user_id: string;
  username: string;
  watch_directory_path: string;
  exists: boolean;
  enabled: boolean;
}

export interface UserWatchDirectoryOperationResponse {
  success: boolean;
  message: string;
  watch_directory_path?: string;
}

export const userWatchService = {
  getUserWatchDirectory: (userId: string) => {
    return api.get<UserWatchDirectoryResponse>(`/users/${userId}/watch-directory`)
  },

  createUserWatchDirectory: (userId: string) => {
    return api.post<UserWatchDirectoryOperationResponse>(`/users/${userId}/watch-directory`)
  },

  deleteUserWatchDirectory: (userId: string) => {
    return api.delete<UserWatchDirectoryOperationResponse>(`/users/${userId}/watch-directory`)
  },
}

export const sourcesService = {
  triggerSync: (sourceId: string) => {
    return api.post(`/sources/${sourceId}/sync`)
  },

  triggerDeepScan: (sourceId: string) => {
    return api.post(`/sources/${sourceId}/deep-scan`)
  },

  stopSync: (sourceId: string) => {
    return api.post(`/sources/${sourceId}/sync/stop`)
  },

  getSyncStatus: (sourceId: string) => {
    return api.get(`/sources/${sourceId}/sync/status`)
  },

  createSyncProgressWebSocket: (sourceId: string) => {
    return new SyncProgressWebSocket(sourceId);
  },
}