// Comprehensive mocking utilities to prevent HTTP requests in tests
import { vi } from 'vitest';

/**
 * Creates a comprehensive axios mock that prevents all HTTP requests
 * This should be used in test files that have components making API calls
 */
export const createComprehensiveAxiosMock = () => {
  const mockAxiosInstance = {
    get: vi.fn().mockResolvedValue({ data: {} }),
    post: vi.fn().mockResolvedValue({ data: { success: true } }),
    put: vi.fn().mockResolvedValue({ data: { success: true } }),
    delete: vi.fn().mockResolvedValue({ data: { success: true } }),
    patch: vi.fn().mockResolvedValue({ data: { success: true } }),
    request: vi.fn().mockResolvedValue({ data: { success: true } }),
    head: vi.fn().mockResolvedValue({ data: {} }),
    options: vi.fn().mockResolvedValue({ data: {} }),
    defaults: { 
      headers: { 
        common: {},
        get: {},
        post: {},
        put: {},
        delete: {},
        patch: {},
      }
    },
    interceptors: {
      request: { use: vi.fn(), eject: vi.fn() },
      response: { use: vi.fn(), eject: vi.fn() },
    },
  };

  return {
    default: {
      create: vi.fn(() => mockAxiosInstance),
      ...mockAxiosInstance,
    },
  };
};

/**
 * Creates comprehensive API service mocks
 */
export const createComprehensiveApiMocks = () => ({
  api: {
    get: vi.fn().mockResolvedValue({ data: {} }),
    post: vi.fn().mockResolvedValue({ data: { success: true } }),
    put: vi.fn().mockResolvedValue({ data: { success: true } }),
    delete: vi.fn().mockResolvedValue({ data: { success: true } }),
    patch: vi.fn().mockResolvedValue({ data: { success: true } }),
    defaults: { headers: { common: {} } },
  },
  documentService: {
    getRetryRecommendations: vi.fn().mockResolvedValue({ 
      data: { recommendations: [], total_recommendations: 0 } 
    }),
    bulkRetryOcr: vi.fn().mockResolvedValue({ data: { success: true } }),
    getFailedDocuments: vi.fn().mockResolvedValue({
      data: {
        documents: [],
        pagination: { total: 0, limit: 25, offset: 0, total_pages: 1 },
        statistics: { total_failed: 0, by_reason: {}, by_stage: {} },
      },
    }),
    getDuplicates: vi.fn().mockResolvedValue({
      data: {
        duplicates: [],
        pagination: { total: 0, limit: 25, offset: 0, has_more: false },
        statistics: { total_duplicate_groups: 0 },
      },
    }),
    enhancedSearch: vi.fn().mockResolvedValue({ 
      data: { 
        documents: [], 
        total: 0, 
        query_time_ms: 0, 
        suggestions: [] 
      } 
    }),
    search: vi.fn().mockResolvedValue({ 
      data: { 
        documents: [], 
        total: 0, 
        query_time_ms: 0, 
        suggestions: [] 
      } 
    }),
    getOcrText: vi.fn().mockResolvedValue({ data: {} }),
    upload: vi.fn().mockResolvedValue({ data: {} }),
    list: vi.fn().mockResolvedValue({ data: [] }),
    listWithPagination: vi.fn().mockResolvedValue({ 
      data: { 
        documents: [], 
        pagination: { total: 0, limit: 20, offset: 0, has_more: false } 
      } 
    }),
    delete: vi.fn().mockResolvedValue({}),
    bulkDelete: vi.fn().mockResolvedValue({}),
    retryOcr: vi.fn().mockResolvedValue({}),
    getFacets: vi.fn().mockResolvedValue({ data: { mime_types: [], tags: [] } }),
    download: vi.fn().mockResolvedValue({ data: new Blob() }),
    deleteLowConfidence: vi.fn().mockResolvedValue({ data: {} }),
    deleteFailedOcr: vi.fn().mockResolvedValue({ data: {} }),
    view: vi.fn().mockResolvedValue({ data: new Blob() }),
    getThumbnail: vi.fn().mockResolvedValue({ data: new Blob() }),
    getProcessedImage: vi.fn().mockResolvedValue({ data: new Blob() }),
    downloadFile: vi.fn().mockResolvedValue(undefined),
    getRetryStats: vi.fn().mockResolvedValue({ data: {} }),
    getDocumentRetryHistory: vi.fn().mockResolvedValue({ data: [] }),
  },
  queueService: {
    getQueueStatus: vi.fn().mockResolvedValue({ data: { active: 0, waiting: 0 } }),
  },
});

/**
 * Standard pattern for mocking both axios and API services
 * Use this at the top of test files that have components making API calls
 */
export const setupHttpMocks = () => {
  // Mock axios comprehensively
  vi.mock('axios', () => createComprehensiveAxiosMock());
  
  // Mock API services
  const apiMocks = createComprehensiveApiMocks();
  
  return apiMocks;
};