import { vi } from 'vitest';

/**
 * Shape of the services barrel as the document feature uses it. Test files pass this factory
 * to `vi.mock('../../../services/api', …)` and set return values in `beforeEach` (the global
 * setup resets every mock before each test).
 */
export function createApiMock() {
  const api = { get: vi.fn(), put: vi.fn(), post: vi.fn(), delete: vi.fn() };
  return {
    default: api,
    api,
    documentService: {
      getById: vi.fn(),
      getOcrText: vi.fn(),
      view: vi.fn(),
      download: vi.fn(),
      delete: vi.fn(),
      bulkRetryOcr: vi.fn(),
      retryOcr: vi.fn(),
      getProcessedImage: vi.fn(),
      getThumbnail: vi.fn(),
      getDocumentRetryHistory: vi.fn(),
    },
    commentsService: {
      list: vi.fn(),
      getReplies: vi.fn(),
      create: vi.fn(),
      update: vi.fn(),
      delete: vi.fn(),
      getCount: vi.fn(),
    },
    sharedLinksService: {
      create: vi.fn(),
      listAll: vi.fn(),
      listByDocument: vi.fn(),
      revoke: vi.fn(),
    },
    labelService: {
      list: vi.fn(),
      create: vi.fn(),
      getDocumentLabels: vi.fn(),
      setDocumentLabels: vi.fn(),
    },
    sourcesService: {
      list: vi.fn(),
    },
    sharedLinksPublicService: {
      getMetadata: vi.fn(),
      verifyPassword: vi.fn(),
      downloadDocument: vi.fn(),
      viewDocument: vi.fn(),
    },
  };
}

export type ApiMock = ReturnType<typeof createApiMock>;

/** Default happy responses for everything the page touches besides the document itself. */
export function primeApi(m: ApiMock) {
  m.default.get.mockResolvedValue({ status: 200, data: [] });
  m.default.put.mockResolvedValue({ status: 200, data: {} });
  m.default.post.mockResolvedValue({ status: 200, data: {} });
  m.documentService.view.mockResolvedValue({ data: new Blob(['%PDF']) });
  m.documentService.getThumbnail.mockRejectedValue(new Error('none'));
  m.documentService.getDocumentRetryHistory.mockResolvedValue({
    data: { document_id: 'doc-1', retry_history: [], total_retries: 0 },
  });
  m.commentsService.list.mockResolvedValue({ data: [] });
  m.labelService.list.mockResolvedValue({ data: [] });
  m.labelService.getDocumentLabels.mockResolvedValue({ data: [] });
  m.labelService.setDocumentLabels.mockResolvedValue({ data: {} });
  m.sharedLinksService.listByDocument.mockResolvedValue({ data: [] });
  m.sourcesService.list.mockResolvedValue({ data: [] });
  window.localStorage.clear();
}

/** An axios-like rejection with an HTTP status. */
export function httpError(status: number, error?: string) {
  return Object.assign(new Error(`HTTP ${status}`), { response: { status, data: error ? { error } : {} } });
}
