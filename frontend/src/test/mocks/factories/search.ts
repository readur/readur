/**
 * Search factory for generating realistic search responses and related data
 */

import { faker } from '@faker-js/faker'
import { 
  SearchResponse, 
  SearchRequest, 
  EnhancedDocument, 
  SearchSnippet,
  HighlightRange,
  MockSearchResponse,
  FactoryOptions 
} from '../api/types'
import { createMockDocument, createMockDocuments } from './document'

faker.seed(12345)

const SEARCH_MODES = ['simple', 'phrase', 'fuzzy', 'boolean'] as const

/**
 * Create a mock search snippet with highlighting
 */
export const createMockSearchSnippet = (
  text?: string,
  overrides: Partial<SearchSnippet> = {}
): SearchSnippet => {
  const snippetText = text || faker.lorem.paragraphs(2, '\n\n')
  const words = snippetText.split(' ')
  const highlightCount = faker.number.int({ min: 1, max: 3 })
  
  // Generate realistic highlight ranges
  const highlight_ranges: HighlightRange[] = []
  for (let i = 0; i < highlightCount; i++) {
    const wordIndex = faker.number.int({ min: 0, max: words.length - 1 })
    const word = words[wordIndex]
    const start = snippetText.indexOf(word, i > 0 ? highlight_ranges[i-1].end : 0)
    
    if (start !== -1) {
      highlight_ranges.push({
        start,
        end: start + word.length,
      })
    }
  }

  return {
    text: snippetText,
    start_offset: faker.number.int({ min: 0, max: 1000 }),
    end_offset: faker.number.int({ min: 1001, max: 2000 }),
    highlight_ranges: highlight_ranges.sort((a, b) => a.start - b.start),
    ...overrides,
  }
}

/**
 * Create a mock enhanced document with search data
 */
export const createMockEnhancedDocument = (
  overrides: Partial<EnhancedDocument> = {}
): EnhancedDocument => {
  const baseDoc = createMockDocument()
  const snippetCount = faker.number.int({ min: 1, max: 4 })
  
  return {
    ...baseDoc,
    search_rank: faker.number.float({ min: 0.1, max: 1.0, fractionDigits: 4 }),
    snippets: Array.from({ length: snippetCount }, () => createMockSearchSnippet()),
    ...overrides,
  }
}

/**
 * Create a mock search request
 */
export const createMockSearchRequest = (overrides: Partial<SearchRequest> = {}): SearchRequest => ({
  query: faker.lorem.words(3),
  tags: faker.helpers.maybe(() => 
    faker.helpers.arrayElements(['important', 'work', 'personal'], { min: 1, max: 2 })
  ),
  mime_types: faker.helpers.maybe(() =>
    faker.helpers.arrayElements(['application/pdf', 'image/jpeg', 'text/plain'], { min: 1, max: 2 })
  ),
  limit: faker.number.int({ min: 10, max: 100 }),
  offset: faker.number.int({ min: 0, max: 50 }),
  include_snippets: faker.datatype.boolean(0.8),
  snippet_length: faker.number.int({ min: 100, max: 500 }),
  search_mode: faker.helpers.arrayElement(SEARCH_MODES),
  ...overrides,
})

/**
 * Create a mock search response
 */
export const createMockSearchResponse = (
  overrides: Partial<MockSearchResponse> = {}
): MockSearchResponse => {
  const documentCount = faker.number.int({ min: 0, max: 25 })
  const total = faker.number.int({ min: documentCount, max: 1000 })
  
  return {
    documents: Array.from({ length: documentCount }, () => createMockEnhancedDocument()),
    total,
    query_time_ms: faker.number.int({ min: 5, max: 500 }),
    suggestions: faker.helpers.arrayElements([
      'invoice',
      'report',
      'contract',
      'presentation',
      'meeting notes',
      'financial',
      'quarterly',
      'annual',
    ], { min: 0, max: 5 }),
    ...overrides,
  }
}

/**
 * Create a search response with specific characteristics
 */
export const createMockSearchResponseWithScenario = (scenario: string): MockSearchResponse => {
  const baseResponse = createMockSearchResponse()

  switch (scenario) {
    case 'no_results':
      return {
        ...baseResponse,
        documents: [],
        total: 0,
        query_time_ms: faker.number.int({ min: 1, max: 10 }),
        suggestions: [
          'Try a different search term',
          'Check your spelling',
          'Use fewer filters',
        ],
      }

    case 'single_result':
      return {
        ...baseResponse,
        documents: [createMockEnhancedDocument()],
        total: 1,
        suggestions: [],
      }

    case 'many_results':
      return {
        ...baseResponse,
        documents: Array.from({ length: 25 }, () => createMockEnhancedDocument()),
        total: 1250,
        query_time_ms: faker.number.int({ min: 50, max: 200 }),
      }

    case 'pdf_only_results':
      const pdfDocs = Array.from({ length: 5 }, () => 
        createMockEnhancedDocument({ 
          mime_type: 'application/pdf',
          filename: `${faker.lorem.words(2)}.pdf`,
        })
      )
      return {
        ...baseResponse,
        documents: pdfDocs,
        total: 5,
      }

    case 'high_confidence_results':
      const highConfidenceDocs = Array.from({ length: 8 }, () =>
        createMockEnhancedDocument({
          search_rank: faker.number.float({ min: 0.8, max: 1.0, fractionDigits: 4 }),
          ocr_confidence: faker.number.float({ min: 0.9, max: 1.0, fractionDigits: 3 }),
        })
      )
      return {
        ...baseResponse,
        documents: highConfidenceDocs,
        total: 8,
      }

    case 'slow_search':
      return {
        ...baseResponse,
        query_time_ms: faker.number.int({ min: 2000, max: 5000 }),
      }

    case 'with_suggestions':
      return {
        ...baseResponse,
        documents: [],
        total: 0,
        suggestions: [
          'invoice 2024',
          'invoice receipt',
          'invoice template',
          'quarterly invoice',
        ],
      }

    default:
      console.warn(`Unknown search scenario: ${scenario}`)
      return baseResponse
  }
}

/**
 * Create search responses for specific test scenarios
 */
export const createSearchScenarios = (): Record<string, MockSearchResponse[]> => ({
  empty: [createMockSearchResponseWithScenario('no_results')],
  single: [createMockSearchResponseWithScenario('single_result')],
  many: [createMockSearchResponseWithScenario('many_results')],
  pdf_results: [createMockSearchResponseWithScenario('pdf_only_results')],
  high_confidence: [createMockSearchResponseWithScenario('high_confidence_results')],
  with_suggestions: [createMockSearchResponseWithScenario('with_suggestions')],
  performance_scenarios: [
    createMockSearchResponse({ query_time_ms: 10 }), // Fast
    createMockSearchResponseWithScenario('slow_search'), // Slow
  ],
})

/**
 * Create faceted search data
 */
export const createMockSearchFacets = () => ({
  mime_types: [
    { value: 'application/pdf', count: 1250 },
    { value: 'image/jpeg', count: 890 },
    { value: 'text/plain', count: 456 },
    { value: 'image/png', count: 234 },
    { value: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document', count: 123 },
  ],
  tags: [
    { value: 'important', count: 567 },
    { value: 'work', count: 432 },
    { value: 'personal', count: 345 },
    { value: 'invoice', count: 234 },
    { value: 'receipt', count: 123 },
  ],
})

/**
 * Reset faker seed for consistent test results
 */
export const resetSearchFactorySeed = (seed: number = 12345) => {
  faker.seed(seed)
}