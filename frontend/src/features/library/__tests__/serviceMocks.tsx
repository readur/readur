/**
 * Shared service mocks and render helpers for the Library tests. Each test file wires the mocks
 * with:
 *
 *   vi.mock('../../../services/api', async () => (await import('./serviceMocks')).apiModule());
 *   vi.mock('../../../services/api/labels', async () => (await import('./serviceMocks')).labelsModule());
 *   vi.mock('../../../components/BulkRetryModal', async () => (await import('./serviceMocks')).retryModalModule());
 *
 * and calls `setupLibraryMocks()` in `beforeEach` (the global setup resets every mock).
 */
import { vi } from 'vitest';
import type { DocumentResponse, EnhancedDocumentResponse, Label } from '../../../types/generated';
import { acknowledgeAll, flushLit } from '../../board/litStore';
import { resetThumbnailLoader } from '../../document/thumbnailLoader';

export const documentService = {
  listFiltered: vi.fn(),
  getFacets: vi.fn(),
  getOcrText: vi.fn(),
  getThumbnail: vi.fn(),
  downloadFile: vi.fn(),
  retryOcr: vi.fn(),
  delete: vi.fn(),
  bulkDelete: vi.fn(),
  enhancedSearch: vi.fn(),
  getById: vi.fn(),
};
export const searchService = { enhancedSearch: vi.fn(), getTimeline: vi.fn() };
export const sharedLinksService = { listByDocument: vi.fn(), create: vi.fn(), revoke: vi.fn() };
export const apiClient = { get: vi.fn(), post: vi.fn(), put: vi.fn(), delete: vi.fn() };
export const labelService = {
  list: vi.fn(),
  create: vi.fn(),
  update: vi.fn(),
  setDocumentLabels: vi.fn(),
  bulkAssign: vi.fn(),
};
export const retryModal = vi.fn();

export function apiModule() {
  return { default: apiClient, api: apiClient, documentService, searchService, sharedLinksService };
}

export function labelsModule() {
  return { labelService };
}

export function retryModalModule() {
  return {
    BulkRetryModal: (props: { open: boolean; selectedDocumentIds?: string[] }) => {
      retryModal(props);
      return props.open ? (
        <div role="dialog" aria-label="Retry OCR">
          {(props.selectedDocumentIds ?? []).length} selected for retry
        </div>
      ) : null;
    },
  };
}

export const label = (id: string, name: string, extra: Partial<Label> = {}): Label => ({
  id,
  user_id: 'u1',
  name,
  description: null,
  color: '#0969da',
  background_color: null,
  icon: null,
  is_system: false,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
  document_count: 0,
  source_count: 0,
  ...extra,
});

export const LABELS = [label('l-tax', 'Tax'), label('l-home', 'Home'), label('l-work', 'Work')];

export const doc = (id: string, name: string, extra: Partial<DocumentResponse> = {}): DocumentResponse => ({
  id,
  filename: name,
  original_filename: name,
  file_path: `/files/${name}`,
  file_size: 2048,
  mime_type: 'application/pdf',
  tags: [],
  labels: [],
  created_at: '2026-09-20T10:00:00Z',
  updated_at: '2026-09-21T10:00:00Z',
  user_id: 'u1',
  has_ocr_text: true,
  ocr_confidence: 91.4,
  ocr_word_count: 120,
  ocr_processing_time_ms: 800,
  ocr_status: 'completed',
  ...extra,
});

export const hit = (d: DocumentResponse, snippet: string, ranges: [number, number][]): EnhancedDocumentResponse => ({
  id: d.id,
  filename: d.filename,
  original_filename: d.original_filename,
  file_size: d.file_size,
  mime_type: d.mime_type,
  tags: [],
  labels: d.labels,
  created_at: d.created_at,
  updated_at: d.updated_at,
  source_id: d.source_id ?? null,
  source_type: d.source_type ?? null,
  has_ocr_text: true,
  ocr_confidence: d.ocr_confidence,
  ocr_word_count: d.ocr_word_count,
  ocr_processing_time_ms: d.ocr_processing_time_ms,
  ocr_status: d.ocr_status,
  search_rank: 0.9,
  snippets: [
    { text: snippet, start_offset: 0, end_offset: snippet.length, highlight_ranges: ranges.map(([start, end]) => ({ start, end })) },
  ],
});

export const DOCS = [
  doc('d1', 'invoice-march.pdf', { labels: [LABELS[0], LABELS[1], LABELS[2]] }),
  doc('d2', 'lease.pdf', { source_id: 's1', source_type: 'webdav', ocr_status: 'failed' }),
  doc('d3', 'photo.png', { mime_type: 'image/png', ocr_status: 'processing', ocr_progress_current: 3, ocr_progress_total: 12 }),
];

export const listResponse = (documents: DocumentResponse[], total = documents.length) => ({
  data: { documents, pagination: { total, limit: 50, offset: 0, has_more: false } },
});

export const searchResponse = (documents: EnhancedDocumentResponse[], total = documents.length) => ({
  data: { documents, total, query_time_ms: 5, suggestions: [] },
});

class MemoryStorage implements Storage {
  private data = new Map<string, string>();
  get length() {
    return this.data.size;
  }
  clear() {
    this.data.clear();
  }
  getItem(key: string) {
    return this.data.get(key) ?? null;
  }
  key(i: number) {
    return Array.from(this.data.keys())[i] ?? null;
  }
  removeItem(key: string) {
    this.data.delete(key);
  }
  setItem(key: string, value: string) {
    this.data.set(key, String(value));
  }
}

/** Default happy-path responses; tests override what they need. */
export function setupLibraryMocks() {
  Object.defineProperty(window, 'localStorage', { value: new MemoryStorage(), configurable: true, writable: true });
  // Most Library tests read the table; the grid tests switch the layout themselves.
  window.localStorage.setItem('readur.library.view', 'table');
  acknowledgeAll();
  flushLit();
  resetThumbnailLoader();
  documentService.listFiltered.mockResolvedValue(listResponse(DOCS, 3));
  documentService.getFacets.mockResolvedValue({
    data: { mime_types: [{ value: 'application/pdf', count: 2 }, { value: 'image/png', count: 1 }], tags: [] },
  });
  documentService.getOcrText.mockResolvedValue({
    data: {
      id: 'd1',
      filename: 'invoice-march.pdf',
      has_ocr_text: true,
      ocr_text: 'Invoice for March. Total due: 120 EUR.',
      ocr_confidence: 91.4,
      ocr_status: 'completed',
      ocr_processing_time_ms: 800,
      detected_language: 'eng',
      pages_processed: 2,
    },
  });
  documentService.getThumbnail.mockRejectedValue(new Error('no thumbnail'));
  documentService.downloadFile.mockResolvedValue(undefined);
  documentService.retryOcr.mockResolvedValue({ data: {} });
  documentService.delete.mockResolvedValue({ data: {} });
  // Like the API: every requested document deleted.
  documentService.bulkDelete.mockImplementation(async (ids: string[]) => ({
    data: { success: true, deleted_count: ids.length, failed_count: 0, deleted_documents: ids },
  }));
  searchService.enhancedSearch.mockResolvedValue(searchResponse([]));
  searchService.getTimeline.mockResolvedValue({ data: [] });
  sharedLinksService.listByDocument.mockResolvedValue({ data: [] });
  apiClient.get.mockImplementation((url: string) =>
    Promise.resolve({ data: url === '/sources' ? [{ id: 's1', name: 'Office NAS', source_type: 'webdav' }] : [] }),
  );
  labelService.list.mockResolvedValue({ data: LABELS });
  labelService.create.mockImplementation((draft: { name: string }) => Promise.resolve({ data: label('l-new', draft.name) }));
  labelService.setDocumentLabels.mockResolvedValue({ data: {} });
  labelService.bulkAssign.mockResolvedValue({ data: {} });
}

