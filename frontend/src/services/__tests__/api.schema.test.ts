import { describe, test, expect } from 'vitest';

// Type definitions for API responses to ensure consistency
interface FailureCategory {
  reason: string;
  display_name: string;
  count: number;
}

interface FailedOcrStatistics {
  total_failed: number;
  failure_categories: FailureCategory[];
}

interface FailedOcrResponse {
  documents: any[];
  pagination: {
    total: number;
    limit: number;
    offset: number;
    has_more: boolean;
  };
  statistics: FailedOcrStatistics;
}

describe('API Response Schema Validation', () => {
  describe('FailedOcrResponse Schema', () => {
    test('validates complete valid response structure', () => {
      const validResponse: FailedOcrResponse = {
        documents: [],
        pagination: {
          total: 0,
          limit: 25,
          offset: 0,
          has_more: false,
        },
        statistics: {
          total_failed: 0,
          failure_categories: [
            {
              reason: 'low_ocr_confidence',
              display_name: 'Low OCR Confidence',
              count: 5,
            },
            {
              reason: 'pdf_parsing_error',
              display_name: 'PDF Parsing Error',
              count: 2,
            },
          ],
        },
      };

      expect(validateFailedOcrResponse(validResponse)).toBe(true);
    });

    test('validates response with empty failure_categories', () => {
      const responseWithEmptyCategories: FailedOcrResponse = {
        documents: [],
        pagination: {
          total: 0,
          limit: 25,
          offset: 0,
          has_more: false,
        },
        statistics: {
          total_failed: 0,
          failure_categories: [],
        },
      };

      expect(validateFailedOcrResponse(responseWithEmptyCategories)).toBe(true);
    });

    test('catches missing required fields', () => {
      const invalidResponses = [
        // Missing documents
        {
          pagination: { total: 0, limit: 25, offset: 0, has_more: false },
          statistics: { total_failed: 0, failure_categories: [] },
        },
        // Missing pagination
        {
          documents: [],
          statistics: { total_failed: 0, failure_categories: [] },
        },
        // Missing statistics
        {
          documents: [],
          pagination: { total: 0, limit: 25, offset: 0, has_more: false },
        },
        // Missing statistics.failure_categories
        {
          documents: [],
          pagination: { total: 0, limit: 25, offset: 0, has_more: false },
          statistics: { total_failed: 0 },
        },
      ];

      for (const invalidResponse of invalidResponses) {
        expect(validateFailedOcrResponse(invalidResponse as any)).toBe(false);
      }
    });

    test('catches null/undefined critical fields', () => {
      const nullFieldResponses = [
        {
          documents: [],
          pagination: { total: 0, limit: 25, offset: 0, has_more: false },
          statistics: null, // This was our original bug
        },
        {
          documents: [],
          pagination: { total: 0, limit: 25, offset: 0, has_more: false },
          statistics: {
            total_failed: 0,
            failure_categories: null, // This could also cause issues
          },
        },
        {
          documents: null,
          pagination: { total: 0, limit: 25, offset: 0, has_more: false },
          statistics: { total_failed: 0, failure_categories: [] },
        },
      ];

      for (const nullResponse of nullFieldResponses) {
        expect(validateFailedOcrResponse(nullResponse as any)).toBe(false);
      }
    });

    test('validates failure category structure', () => {
      const invalidCategoryStructures = [
        // Missing required fields in category
        {
          documents: [],
          pagination: { total: 0, limit: 25, offset: 0, has_more: false },
          statistics: {
            total_failed: 1,
            failure_categories: [
              { reason: 'test', count: 1 }, // Missing display_name
            ],
          },
        },
        // Wrong type for count
        {
          documents: [],
          pagination: { total: 0, limit: 25, offset: 0, has_more: false },
          statistics: {
            total_failed: 1,
            failure_categories: [
              { reason: 'test', display_name: 'Test', count: 'not a number' },
            ],
          },
        },
      ];

      for (const invalidStructure of invalidCategoryStructures) {
        expect(validateFailedOcrResponse(invalidStructure as any)).toBe(false);
      }
    });
  });

  describe('Frontend Safety Helpers', () => {
    test('safe array access helper works correctly', () => {
      const responses = [
        { failure_categories: [{ reason: 'test', display_name: 'Test', count: 1 }] },
        { failure_categories: [] },
        { failure_categories: null },
        { failure_categories: undefined },
        {},
        null,
        undefined,
      ];

      for (const response of responses) {
        const result = safeGetFailureCategories(response);
        expect(Array.isArray(result)).toBe(true);
        expect(result.length).toBeGreaterThanOrEqual(0);
      }
    });

    test('safe statistics access helper works correctly', () => {
      const responses = [
        { statistics: { total_failed: 5, failure_categories: [] } },
        { statistics: null },
        { statistics: undefined },
        {},
        null,
        undefined,
      ];

      for (const response of responses) {
        const result = safeGetStatistics(response);
        expect(typeof result.total_failed).toBe('number');
        expect(Array.isArray(result.failure_categories)).toBe(true);
      }
    });
  });
});

// Validation functions that could be used in production code
function validateFailedOcrResponse(response: any): response is FailedOcrResponse {
  if (!response || typeof response !== 'object') {
    return false;
  }

  // Check required top-level fields
  if (!Array.isArray(response.documents)) {
    return false;
  }

  if (!response.pagination || typeof response.pagination !== 'object') {
    return false;
  }

  if (!response.statistics || typeof response.statistics !== 'object') {
    return false;
  }

  // Check pagination structure
  const { pagination } = response;
  if (
    typeof pagination.total !== 'number' ||
    typeof pagination.limit !== 'number' ||
    typeof pagination.offset !== 'number' ||
    typeof pagination.has_more !== 'boolean'
  ) {
    return false;
  }

  // Check statistics structure
  const { statistics } = response;
  if (
    typeof statistics.total_failed !== 'number' ||
    !Array.isArray(statistics.failure_categories)
  ) {
    return false;
  }

  // Check each failure category structure
  for (const category of statistics.failure_categories) {
    if (
      !category ||
      typeof category.reason !== 'string' ||
      typeof category.display_name !== 'string' ||
      typeof category.count !== 'number'
    ) {
      return false;
    }
  }

  return true;
}

// Helper functions for safe access (these could be used in components)
function safeGetFailureCategories(response: any): FailureCategory[] {
  if (
    response &&
    response.statistics &&
    Array.isArray(response.statistics.failure_categories)
  ) {
    return response.statistics.failure_categories;
  }
  return [];
}

function safeGetStatistics(response: any): FailedOcrStatistics {
  const defaultStats: FailedOcrStatistics = {
    total_failed: 0,
    failure_categories: [],
  };

  if (
    response &&
    response.statistics &&
    typeof response.statistics === 'object'
  ) {
    return {
      total_failed: typeof response.statistics.total_failed === 'number' 
        ? response.statistics.total_failed 
        : 0,
      failure_categories: Array.isArray(response.statistics.failure_categories)
        ? response.statistics.failure_categories
        : [],
    };
  }

  return defaultStats;
}

// Export helpers for use in production code
export { validateFailedOcrResponse, safeGetFailureCategories, safeGetStatistics };