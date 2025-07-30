import { vi } from 'vitest'

// Mock axios instance
export const api = {
  defaults: { headers: { common: {} } },
  get: vi.fn(),
  post: vi.fn(),
  put: vi.fn(),
  delete: vi.fn(),
}

// Mock document service
export const documentService = {
  list: vi.fn(),
  getById: vi.fn(),
  getOcrText: vi.fn(),
  upload: vi.fn(),
  delete: vi.fn(),
  search: vi.fn(),
  enhancedSearch: vi.fn(),
  download: vi.fn(),
  getThumbnail: vi.fn(),
  getProcessedImage: vi.fn(),
  updateTags: vi.fn(),
  getFailedOcrDocuments: vi.fn(),
  getDuplicates: vi.fn(),
  retryOcr: vi.fn(),
  deleteLowConfidence: vi.fn(),
  getDocumentRetryHistory: vi.fn(),
  getRetryRecommendations: vi.fn(),
  getRetryStats: vi.fn(),
  bulkRetryOcr: vi.fn(),
}

// Mock WebSocket constants  
const WEBSOCKET_CONNECTING = 0;
const WEBSOCKET_OPEN = 1;
const WEBSOCKET_CLOSING = 2;
const WEBSOCKET_CLOSED = 3;

// Create a proper WebSocket mock factory
const createMockWebSocket = () => {
  const mockInstance = {
    onopen: null as ((event: Event) => void) | null,
    onmessage: null as ((event: MessageEvent) => void) | null,
    onerror: null as ((event: Event) => void) | null,
    onclose: null as ((event: CloseEvent) => void) | null,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    send: vi.fn(),
    close: vi.fn(),
    readyState: WEBSOCKET_CONNECTING,
    url: '',
    protocol: '',
    extensions: '',
    bufferedAmount: 0,
    binaryType: 'blob' as BinaryType,
    CONNECTING: WEBSOCKET_CONNECTING,
    OPEN: WEBSOCKET_OPEN,
    CLOSING: WEBSOCKET_CLOSING,
    CLOSED: WEBSOCKET_CLOSED,
    dispatchEvent: vi.fn(),
  };
  return mockInstance;
};

// Create the main mock instance
let currentMockWebSocket = createMockWebSocket();

// Mock the global WebSocket
global.WebSocket = vi.fn(() => currentMockWebSocket) as any;
(global.WebSocket as any).CONNECTING = WEBSOCKET_CONNECTING;
(global.WebSocket as any).OPEN = WEBSOCKET_OPEN;
(global.WebSocket as any).CLOSING = WEBSOCKET_CLOSING;
(global.WebSocket as any).CLOSED = WEBSOCKET_CLOSED;

// Mock SyncProgressWebSocket class
export class MockSyncProgressWebSocket {
  private listeners: { [key: string]: ((data: any) => void)[] } = {};
  
  constructor(private sourceId: string) {
    // Store reference to current instance for test access
    currentMockSyncProgressWebSocket = this;
  }

  connect(): Promise<void> {
    // Simulate successful connection
    setTimeout(() => {
      this.emit('connectionStatus', 'connected');
    }, 10);
    return Promise.resolve();
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
    this.listeners = {};
  }

  getReadyState(): number {
    return WEBSOCKET_OPEN;
  }

  isConnected(): boolean {
    return true;
  }

  // Test helper methods
  simulateProgress(data: any): void {
    this.emit('progress', data);
  }

  simulateHeartbeat(data: any): void {
    this.emit('heartbeat', data);
  }

  simulateError(data: any): void {
    this.emit('error', data);
  }

  simulateConnectionStatus(status: string): void {
    this.emit('connectionStatus', status);
  }
}

// Create current mock instance holder
let currentMockSyncProgressWebSocket: MockSyncProgressWebSocket | null = null;

// Mock sources service
export const sourcesService = {
  triggerSync: vi.fn(),
  triggerDeepScan: vi.fn(),
  stopSync: vi.fn(),
  getSyncStatus: vi.fn(),
  createSyncProgressWebSocket: vi.fn((sourceId: string) => {
    return new MockSyncProgressWebSocket(sourceId);
  }),
}

// Export helper functions for tests
export const getMockWebSocket = () => currentMockWebSocket;
export const getMockSyncProgressWebSocket = () => currentMockSyncProgressWebSocket;

export const resetMockWebSocket = () => {
  currentMockWebSocket = createMockWebSocket();
  // Update global WebSocket mock to return the new instance
  global.WebSocket = vi.fn(() => currentMockWebSocket) as any;
  (global.WebSocket as any).CONNECTING = WEBSOCKET_CONNECTING;
  (global.WebSocket as any).OPEN = WEBSOCKET_OPEN;
  (global.WebSocket as any).CLOSING = WEBSOCKET_CLOSING;
  (global.WebSocket as any).CLOSED = WEBSOCKET_CLOSED;
  return currentMockWebSocket;
};

export const resetMockSyncProgressWebSocket = () => {
  currentMockSyncProgressWebSocket = null;
  return currentMockSyncProgressWebSocket;
};

// Re-export types that components might need
export interface Document {
  id: string
  filename: string
  original_filename: string
  file_size: number
  mime_type: string
  tags: string[]
  created_at: string
  updated_at?: string
  user_id?: string
  file_hash?: string
  has_ocr_text: boolean
  ocr_confidence?: number
  ocr_word_count?: number
  ocr_processing_time_ms?: number
  ocr_status?: string
  // New metadata fields
  original_created_at?: string
  original_modified_at?: string
  source_metadata?: Record<string, any>
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

export default api