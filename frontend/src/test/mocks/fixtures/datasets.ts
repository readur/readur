/**
 * Predefined datasets for consistent testing
 * Provides realistic data combinations for various test cases
 */

import { 
  createMockDocuments,
  createMockDocumentWithScenario,
  createMockUsers,
  createMockSources,
  createDefaultLabels,
  createMockQueueStats,
  getDefaultTestUser,
  getDefaultAdminUser,
} from '../factories'

/**
 * Document datasets for different testing scenarios
 */
export const DOCUMENT_DATASETS = {
  EMPTY: [],
  
  SINGLE_PDF: [
    createMockDocumentWithScenario('pdf_with_high_confidence_ocr')
  ],
  
  MIXED_TYPES: [
    createMockDocumentWithScenario('pdf_with_high_confidence_ocr'),
    createMockDocumentWithScenario('image_with_failed_ocr'),
    createMockDocumentWithScenario('text_file_no_ocr'),
  ],
  
  LARGE_SET: createMockDocuments(100),
  
  OCR_ISSUES: [
    createMockDocumentWithScenario('image_with_failed_ocr'),
    createMockDocumentWithScenario('large_pdf_pending_ocr'),
    ...Array.from({ length: 5 }, () => 
      createMockDocumentWithScenario('image_with_failed_ocr')
    ),
  ],
  
  RECENT_UPLOADS: [
    createMockDocumentWithScenario('recently_uploaded'),
    createMockDocumentWithScenario('recently_uploaded'),
    createMockDocumentWithScenario('recently_uploaded'),
  ],
  
  DUPLICATES: [
    createMockDocumentWithScenario('duplicate_document'),
    createMockDocumentWithScenario('duplicate_document'),
  ],
  
  DIFFERENT_SOURCES: [
    createMockDocumentWithScenario('webdav_synced'),
    createMockDocumentWithScenario('s3_synced'),
    createMockDocumentWithScenario('recently_uploaded'),
  ],
}

/**
 * User datasets for different testing scenarios
 */
export const USER_DATASETS = {
  SINGLE_USER: [getDefaultTestUser()],
  
  ADMIN_AND_USER: [getDefaultAdminUser(), getDefaultTestUser()],
  
  MULTIPLE_USERS: createMockUsers(5),
  
  MIXED_ROLES: [
    getDefaultAdminUser(),
    getDefaultTestUser(),
    ...createMockUsers(3, { overrides: { role: 'user' } }),
  ],
  
  INACTIVE_USERS: [
    getDefaultTestUser(),
    ...createMockUsers(2, { overrides: { is_active: false } }),
  ],
}

/**
 * Source datasets for different testing scenarios
 */
export const SOURCE_DATASETS = {
  NO_SOURCES: [],
  
  SINGLE_LOCAL: [
    createMockSources(1, { 
      overrides: { 
        source_type: 'local_folder',
        enabled: true,
        sync_status: 'idle',
      } 
    })[0]
  ],
  
  MIXED_TYPES: [
    ...createMockSources(1, { overrides: { source_type: 'local_folder' } }),
    ...createMockSources(1, { overrides: { source_type: 'webdav' } }),
    ...createMockSources(1, { overrides: { source_type: 's3' } }),
  ],
  
  MANY_SOURCES: createMockSources(10),
  
  SYNCING_SOURCES: createMockSources(3, { 
    overrides: { sync_status: 'syncing' } 
  }),
  
  ERROR_SOURCES: createMockSources(2, { 
    overrides: { sync_status: 'error' } 
  }),
  
  DISABLED_SOURCES: createMockSources(3, { 
    overrides: { enabled: false } 
  }),
}

/**
 * Label datasets for different testing scenarios
 */
export const LABEL_DATASETS = {
  NO_LABELS: [],
  
  DEFAULT_LABELS: createDefaultLabels(),
  
  HEAVILY_USED: createDefaultLabels().map(label => ({
    ...label,
    document_count: Math.floor(Math.random() * 200) + 50,
  })),
  
  SOME_UNUSED: [
    ...createDefaultLabels().slice(0, 3).map(label => ({
      ...label,
      document_count: Math.floor(Math.random() * 50) + 10,
    })),
    ...createDefaultLabels().slice(3).map(label => ({
      ...label,
      document_count: 0,
    })),
  ],
}

/**
 * Queue datasets for different testing scenarios
 */
export const QUEUE_DATASETS = {
  EMPTY_QUEUE: createMockQueueStats({
    pending_count: 0,
    processing_count: 0,
    failed_count: 0,
    completed_today: 0,
  }),
  
  LIGHT_LOAD: createMockQueueStats({
    pending_count: 5,
    processing_count: 1,
    failed_count: 2,
    completed_today: 45,
  }),
  
  HEAVY_LOAD: createMockQueueStats({
    pending_count: 50,
    processing_count: 5,
    failed_count: 15,
    completed_today: 200,
    avg_wait_time_minutes: 25,
    oldest_pending_minutes: 90,
  }),
  
  BACKLOGGED: createMockQueueStats({
    pending_count: 200,
    processing_count: 2,
    failed_count: 30,
    completed_today: 150,
    avg_wait_time_minutes: 60,
    oldest_pending_minutes: 240,
  }),
  
  MANY_FAILURES: createMockQueueStats({
    pending_count: 10,
    processing_count: 1,
    failed_count: 75,
    completed_today: 50,
  }),
}

/**
 * Complete dataset combinations for comprehensive testing
 */
export const COMPLETE_DATASETS = {
  MINIMAL: {
    documents: DOCUMENT_DATASETS.SINGLE_PDF,
    users: USER_DATASETS.SINGLE_USER,
    sources: SOURCE_DATASETS.SINGLE_LOCAL,
    labels: LABEL_DATASETS.DEFAULT_LABELS,
    queueStats: QUEUE_DATASETS.EMPTY_QUEUE,
  },
  
  TYPICAL: {
    documents: DOCUMENT_DATASETS.MIXED_TYPES,
    users: USER_DATASETS.ADMIN_AND_USER,
    sources: SOURCE_DATASETS.MIXED_TYPES,
    labels: LABEL_DATASETS.DEFAULT_LABELS,
    queueStats: QUEUE_DATASETS.LIGHT_LOAD,
  },
  
  COMPREHENSIVE: {
    documents: DOCUMENT_DATASETS.LARGE_SET,
    users: USER_DATASETS.MULTIPLE_USERS,
    sources: SOURCE_DATASETS.MANY_SOURCES,
    labels: LABEL_DATASETS.HEAVILY_USED,
    queueStats: QUEUE_DATASETS.HEAVY_LOAD,
  },
  
  PROBLEMATIC: {
    documents: DOCUMENT_DATASETS.OCR_ISSUES,
    users: USER_DATASETS.MIXED_ROLES,
    sources: SOURCE_DATASETS.ERROR_SOURCES,
    labels: LABEL_DATASETS.SOME_UNUSED,
    queueStats: QUEUE_DATASETS.MANY_FAILURES,
  },
}

/**
 * Search result datasets for testing search functionality
 */
export const SEARCH_DATASETS = {
  NO_RESULTS: {
    documents: [],
    total: 0,
    query_time_ms: 15,
    suggestions: ['Try a different search term', 'Check your spelling'],
  },
  
  FEW_RESULTS: {
    documents: DOCUMENT_DATASETS.MIXED_TYPES,
    total: 3,
    query_time_ms: 45,
    suggestions: [],
  },
  
  MANY_RESULTS: {
    documents: DOCUMENT_DATASETS.LARGE_SET.slice(0, 20), // First page
    total: 100,
    query_time_ms: 120,
    suggestions: [],
  },
  
  SLOW_RESULTS: {
    documents: DOCUMENT_DATASETS.MIXED_TYPES,
    total: 3,
    query_time_ms: 2500, // Slow search
    suggestions: [],
  },
}

/**
 * Utility functions for working with datasets
 */
export const DATASET_UTILS = {
  /**
   * Get a random subset of documents
   */
  getRandomDocuments: (count: number) => 
    DOCUMENT_DATASETS.LARGE_SET.slice(0, count),
  
  /**
   * Get documents with specific OCR status
   */
  getDocumentsByOcrStatus: (status: string) =>
    DOCUMENT_DATASETS.LARGE_SET.filter(doc => doc.ocr_status === status),
  
  /**
   * Get documents from specific source type
   */
  getDocumentsBySourceType: (sourceType: string) =>
    DOCUMENT_DATASETS.LARGE_SET.filter(doc => doc.source_type === sourceType),
  
  /**
   * Create a dataset with specific characteristics
   */
  createCustomDataset: (options: {
    documentCount?: number
    userCount?: number
    sourceCount?: number
    labelCount?: number
    includeErrors?: boolean
  }) => {
    const {
      documentCount = 10,
      userCount = 2,
      sourceCount = 3,
      labelCount = 5,
      includeErrors = false,
    } = options

    return {
      documents: includeErrors 
        ? [...createMockDocuments(documentCount - 3), ...DOCUMENT_DATASETS.OCR_ISSUES.slice(0, 3)]
        : createMockDocuments(documentCount),
      users: createMockUsers(userCount),
      sources: includeErrors
        ? [...createMockSources(sourceCount - 1), ...SOURCE_DATASETS.ERROR_SOURCES.slice(0, 1)]
        : createMockSources(sourceCount),
      labels: createDefaultLabels().slice(0, labelCount),
      queueStats: includeErrors ? QUEUE_DATASETS.MANY_FAILURES : QUEUE_DATASETS.LIGHT_LOAD,
    }
  },
}