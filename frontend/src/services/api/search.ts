import api from './client'
import { serializeFilterParams, type EnhancedSearchParams } from './filterParams'
import type { SearchResponse } from '../../types/generated'

export type {
  EnhancedDocumentResponse as EnhancedDocument,
  FacetItem,
  HighlightRange,
  SearchFacetsResponse,
  SearchRequest,
  SearchResponse,
  SearchSnippet,
} from '../../types/generated'

export const searchService = {
  /**
   * GET /search/enhanced with the sort/filter contract. List filters accept
   * arrays (sent comma-separated), dates are sent as ISO strings. Defaults
   * match documentService.enhancedSearch: include_snippets=true,
   * snippet_length=200, search_mode=simple.
   */
  enhancedSearch: (params: EnhancedSearchParams = {}) => {
    return api.get<SearchResponse>('/search/enhanced', {
      params: serializeFilterParams({
        ...params,
        include_snippets: params.include_snippets ?? true,
        snippet_length: params.snippet_length ?? 200,
        search_mode: params.search_mode ?? 'simple',
      }),
    })
  },
}
