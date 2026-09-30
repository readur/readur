import api from './client'
import { serializeFilterParams, type EnhancedSearchParams } from './filterParams'
import type { MonthCount, SearchResponse } from '../../types/generated'

export type {
  EnhancedDocumentResponse as EnhancedDocument,
  FacetItem,
  HighlightRange,
  MonthCount,
  SearchFacetsResponse,
  SearchRequest,
  SearchResponse,
  SearchSnippet,
} from '../../types/generated'

/**
 * Params for `searchService.getTimeline`: the enhanced-search query and
 * filters. Paging, snippet and sort params don't change a histogram.
 */
export type SearchTimelineParams = Omit<
  EnhancedSearchParams,
  'limit' | 'offset' | 'include_snippets' | 'snippet_length' | 'sort_by' | 'sort_order'
>

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

  /**
   * GET /search/timeline: match counts per UTC creation month (`YYYY-MM`)
   * across ALL matches of the same query and filters as enhancedSearch,
   * oldest first, months without matches omitted. Like enhancedSearch it
   * needs a query of 2+ characters or at least one filter (400 otherwise).
   */
  getTimeline: (params: SearchTimelineParams = {}) => {
    return api.get<MonthCount[]>('/search/timeline', {
      params: serializeFilterParams({
        ...params,
        search_mode: params.search_mode ?? 'simple',
      }),
    })
  },
}
