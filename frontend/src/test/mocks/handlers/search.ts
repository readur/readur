/**
 * Mock handlers for search-related API endpoints
 */

import { http, HttpResponse } from 'msw'
import { 
  createMockSearchResponse,
  createMockSearchResponseWithScenario,
  createMockSearchFacets,
} from '../factories'
import { 
  applyDelay, 
  shouldFail, 
  createMockResponse, 
  DEFAULT_MOCK_CONFIG,
} from '../utils/config'
import { MockConfig } from '../api/types'

// Mock state for search
let mockConfig: MockConfig = DEFAULT_MOCK_CONFIG

/**
 * Update mock configuration for all search handlers
 */
export const setSearchMockConfig = (config: Partial<MockConfig>) => {
  mockConfig = { ...mockConfig, ...config }
}

/**
 * Reset search mock state
 */
export const resetSearchState = () => {
  mockConfig = DEFAULT_MOCK_CONFIG
}

export const searchHandlers = [
  // GET /api/search - Basic search
  http.get('/api/search', async ({ request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const url = new URL(request.url)
    const query = url.searchParams.get('query') || ''
    const tags = url.searchParams.getAll('tags')
    const mimeTypes = url.searchParams.getAll('mime_types')
    const limit = parseInt(url.searchParams.get('limit') || '20')
    const offset = parseInt(url.searchParams.get('offset') || '0')
    const searchMode = url.searchParams.get('search_mode') || 'simple'

    // Create appropriate response based on query
    let searchResponse
    if (!query.trim()) {
      searchResponse = createMockSearchResponseWithScenario('no_results')
    } else if (query.toLowerCase().includes('invoice')) {
      searchResponse = createMockSearchResponseWithScenario('pdf_only_results')
    } else if (query.toLowerCase().includes('error') || query.toLowerCase().includes('slow')) {
      searchResponse = createMockSearchResponseWithScenario('slow_search')
    } else if (query.length < 3) {
      searchResponse = createMockSearchResponseWithScenario('with_suggestions')
    } else {
      searchResponse = createMockSearchResponse()
    }

    // Apply filters
    if (tags.length > 0 || mimeTypes.length > 0) {
      searchResponse.documents = searchResponse.documents.filter(doc => {
        const tagMatch = tags.length === 0 || tags.some(tag => doc.tags.includes(tag))
        const mimeMatch = mimeTypes.length === 0 || mimeTypes.includes(doc.mime_type)
        return tagMatch && mimeMatch
      })
      searchResponse.total = searchResponse.documents.length
    }

    // Apply pagination
    searchResponse.documents = searchResponse.documents.slice(offset, offset + limit)

    return HttpResponse.json(createMockResponse(searchResponse, mockConfig))
  }),

  // GET /api/search/enhanced - Enhanced search with snippets
  http.get('/api/search/enhanced', async ({ request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const url = new URL(request.url)
    const query = url.searchParams.get('query') || ''
    const includeSnippets = url.searchParams.get('include_snippets') !== 'false'
    const snippetLength = parseInt(url.searchParams.get('snippet_length') || '200')
    const tags = url.searchParams.getAll('tags')
    const mimeTypes = url.searchParams.getAll('mime_types')
    const limit = parseInt(url.searchParams.get('limit') || '20')
    const offset = parseInt(url.searchParams.get('offset') || '0')
    const searchMode = url.searchParams.get('search_mode') || 'simple'

    // Create enhanced response based on query
    let searchResponse
    if (!query.trim()) {
      searchResponse = createMockSearchResponseWithScenario('no_results')
    } else if (query.toLowerCase().includes('high confidence')) {
      searchResponse = createMockSearchResponseWithScenario('high_confidence_results')
    } else if (query.toLowerCase().includes('many') || query.toLowerCase().includes('large')) {
      searchResponse = createMockSearchResponseWithScenario('many_results')
    } else if (query.toLowerCase().includes('pdf')) {
      searchResponse = createMockSearchResponseWithScenario('pdf_only_results')
    } else {
      searchResponse = createMockSearchResponse()
    }

    // Modify snippets based on parameters
    if (includeSnippets) {
      searchResponse.documents.forEach(doc => {
        doc.snippets = doc.snippets.map(snippet => ({
          ...snippet,
          text: snippet.text.substring(0, snippetLength),
        }))
      })
    } else {
      searchResponse.documents.forEach(doc => {
        doc.snippets = []
      })
    }

    // Apply filters
    if (tags.length > 0 || mimeTypes.length > 0) {
      searchResponse.documents = searchResponse.documents.filter(doc => {
        const tagMatch = tags.length === 0 || tags.some(tag => doc.tags.includes(tag))
        const mimeMatch = mimeTypes.length === 0 || mimeTypes.includes(doc.mime_type)
        return tagMatch && mimeMatch
      })
      searchResponse.total = searchResponse.documents.length
    }

    // Apply pagination
    searchResponse.documents = searchResponse.documents.slice(offset, offset + limit)

    return HttpResponse.json(createMockResponse(searchResponse, mockConfig))
  }),

  // GET /api/search/facets - Get search facets
  http.get('/api/search/facets', async () => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const facets = createMockSearchFacets()

    return HttpResponse.json(createMockResponse(facets, mockConfig))
  }),

  // GET /api/search/suggestions - Get search suggestions (autocomplete)
  http.get('/api/search/suggestions', async ({ request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const url = new URL(request.url)
    const query = url.searchParams.get('query') || ''
    const limit = parseInt(url.searchParams.get('limit') || '10')

    const suggestions = []
    
    if (query.toLowerCase().startsWith('inv')) {
      suggestions.push('invoice', 'invoice 2024', 'invoice template', 'inventory')
    } else if (query.toLowerCase().startsWith('rep')) {
      suggestions.push('report', 'report 2024', 'quarterly report', 'annual report')
    } else if (query.toLowerCase().startsWith('con')) {
      suggestions.push('contract', 'contact', 'conference', 'construction')
    } else if (query.toLowerCase().startsWith('doc')) {
      suggestions.push('document', 'documentation', 'doctor', 'dock')
    } else if (query.length >= 2) {
      suggestions.push(
        `${query} 2024`,
        `${query} document`,
        `${query} file`,
        `${query} report`
      )
    }

    return HttpResponse.json(createMockResponse({
      suggestions: suggestions.slice(0, limit),
      query,
    }, mockConfig))
  }),

  // POST /api/search/save - Save search query
  http.post('/api/search/save', async ({ request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const { name, query, filters } = await request.json()

    const savedSearch = {
      id: `saved-search-${Date.now()}`,
      name,
      query,
      filters,
      created_at: new Date().toISOString(),
      user_id: 'current-user-id',
    }

    return HttpResponse.json(createMockResponse(savedSearch, mockConfig))
  }),

  // GET /api/search/saved - Get saved searches
  http.get('/api/search/saved', async () => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const savedSearches = [
      {
        id: 'saved-1',
        name: 'Important Invoices',
        query: 'invoice',
        filters: { tags: ['important'] },
        created_at: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString(),
        user_id: 'current-user-id',
      },
      {
        id: 'saved-2',
        name: 'Work Reports',
        query: 'report',
        filters: { tags: ['work'], mime_types: ['application/pdf'] },
        created_at: new Date(Date.now() - 3 * 24 * 60 * 60 * 1000).toISOString(),
        user_id: 'current-user-id',
      },
    ]

    return HttpResponse.json(createMockResponse({
      saved_searches: savedSearches,
      total: savedSearches.length,
    }, mockConfig))
  }),

  // DELETE /api/search/saved/:id - Delete saved search
  http.delete('/api/search/saved/:id', async ({ params }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    return HttpResponse.json(createMockResponse({
      success: true,
      message: 'Saved search deleted successfully',
    }, mockConfig))
  }),
]