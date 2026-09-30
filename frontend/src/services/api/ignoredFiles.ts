import api from './client'
import type {
  IgnoredFileResponse,
  IgnoredFilesQuery,
  IgnoredFilesStats,
} from '../../types/generated'

export type { IgnoredFileResponse, IgnoredFilesQuery, IgnoredFilesStats } from '../../types/generated'

/** Body of GET /ignored/files. */
export interface IgnoredFilesListResponse {
  ignored_files: IgnoredFileResponse[]
  total: number
  limit?: number
  offset?: number
}

/** Body of the delete endpoints. */
export interface IgnoredFilesDeleteResponse {
  success?: boolean
  message?: string
  deleted_count?: number
}

/** Drops empty filters so they are not sent as `?filename=`. */
function cleanQuery(query: IgnoredFilesQuery): IgnoredFilesQuery {
  const out: IgnoredFilesQuery = {}
  for (const [key, value] of Object.entries(query) as [keyof IgnoredFilesQuery, unknown][]) {
    if (value !== undefined && value !== null && value !== '') {
      ;(out as Record<string, unknown>)[key] = value
    }
  }
  return out
}

/**
 * Files skipped during source syncs. Removing an entry from the list (single or bulk) lets the
 * file be imported again on the next sync.
 */
export const ignoredFilesService = {
  list: (query: IgnoredFilesQuery = {}) => {
    return api.get<IgnoredFilesListResponse>('/ignored/files', { params: cleanQuery(query) })
  },

  stats: () => {
    return api.get<IgnoredFilesStats>('/ignored/files/stats')
  },

  remove: (ignoredFileId: string) => {
    return api.delete<IgnoredFilesDeleteResponse>(`/ignored/files/${ignoredFileId}`)
  },

  bulkRemove: (ignoredFileIds: string[]) => {
    return api.delete<IgnoredFilesDeleteResponse>('/ignored/files/bulk/delete', {
      data: { ignored_file_ids: ignoredFileIds },
    })
  },
}
