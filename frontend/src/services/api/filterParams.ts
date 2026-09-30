import type { PaginationQuery, SearchRequest } from '../../types/generated'

/**
 * The backend takes list filters as ONE comma-separated query value
 * (`label_ids=a,b`); repeated keys are rejected with 400. Callers may pass
 * an array or a ready-made string.
 */
export type CommaList = string | readonly string[]

/** `created_from` / `created_to` are RFC 3339 strings on the wire. */
export type DateInput = string | Date

const LIST_KEYS = ['tags', 'mime_types', 'label_ids', 'source_ids', 'source_types'] as const
const DATE_KEYS = ['created_from', 'created_to'] as const
type ListKey = (typeof LIST_KEYS)[number]
type DateKey = (typeof DATE_KEYS)[number]

type WithTypedFilters<T> = Omit<T, ListKey | DateKey> & {
  [K in ListKey]?: CommaList
} & {
  [K in DateKey]?: DateInput
}

/** Params for `documentService.listFiltered` (GET /documents). */
export type DocumentListParams = WithTypedFilters<PaginationQuery>

/** Params for `searchService.enhancedSearch` (GET /search/enhanced). */
export type EnhancedSearchParams = WithTypedFilters<SearchRequest>

/**
 * Turn typed filter params into the flat query object the backend expects:
 * undefined / null / empty values are dropped, lists are comma-joined, dates
 * are ISO strings. Key order follows the input so the query string is stable.
 */
export function serializeFilterParams(
  params: DocumentListParams | EnhancedSearchParams,
): Record<string, string | number | boolean> {
  const out: Record<string, string | number | boolean> = {}
  for (const [key, value] of Object.entries(params)) {
    if (value === undefined || value === null) continue
    if ((LIST_KEYS as readonly string[]).includes(key)) {
      const joined = typeof value === 'string' ? value : (value as readonly string[]).join(',')
      if (joined !== '') out[key] = joined
    } else if ((DATE_KEYS as readonly string[]).includes(key)) {
      const iso = value instanceof Date ? value.toISOString() : String(value)
      if (iso !== '') out[key] = iso
    } else {
      out[key] = value as string | number | boolean
    }
  }
  return out
}
