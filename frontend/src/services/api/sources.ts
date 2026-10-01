import api from './client'
import { SyncProgressWebSocket } from './syncProgress'
import type {
  CreateSource,
  SourceArrivals,
  SourceResponse,
  TestConnectionRequest,
  UpdateSource,
  WebDAVCrawlEstimate,
} from '../../types/generated'

export type {
  CreateSource,
  DayCount,
  SourceArrivals,
  SourceResponse,
  TestConnectionRequest,
  UpdateSource,
  WebDAVCrawlEstimate,
} from '../../types/generated'

/** Body of POST /sources/test/connection and POST /sources/{id}/validate. */
export interface SourceCheckResponse {
  success: boolean
  message?: string
}

/** Config accepted by POST /sources/estimate (WebDAV only). */
export interface CrawlEstimateConfig {
  server_url: string
  username: string
  password: string
  watch_folders: string[]
  file_extensions: string[]
  auto_sync: boolean
  sync_interval_minutes: number
  server_type: string
}

export const sourcesService = {
  list: () => {
    return api.get<SourceResponse[]>('/sources')
  },

  /**
   * GET /sources/arrivals: one lane per visible source, then the watch folder
   * (key "watch") and uploads (key "upload"), each with `days` zero-filled UTC
   * day counts ending today. The backend accepts 1-60 days (default 14).
   */
  getArrivals: (days?: number) => {
    return api.get<SourceArrivals[]>(
      '/sources/arrivals',
      days === undefined ? undefined : { params: { days } },
    )
  },

  create: (source: CreateSource) => {
    return api.post<SourceResponse>('/sources', source)
  },

  update: (sourceId: string, changes: UpdateSource) => {
    return api.put<SourceResponse>(`/sources/${sourceId}`, changes)
  },

  remove: (sourceId: string) => {
    return api.delete(`/sources/${sourceId}`)
  },

  testConnection: (request: TestConnectionRequest) => {
    return api.post<SourceCheckResponse>('/sources/test/connection', request)
  },

  validate: (sourceId: string) => {
    return api.post<SourceCheckResponse>(`/sources/${sourceId}/validate`)
  },

  /** Crawl estimate for a saved source (routed as GET by the backend). */
  estimate: (sourceId: string) => {
    return api.get<WebDAVCrawlEstimate>(`/sources/${sourceId}/estimate`)
  },

  /** Crawl estimate for an unsaved WebDAV configuration. */
  estimateWithConfig: (config: CrawlEstimateConfig) => {
    return api.post<WebDAVCrawlEstimate>('/sources/estimate', config)
  },

  triggerSync: (sourceId: string) => {
    return api.post(`/sources/${sourceId}/sync`)
  },

  triggerDeepScan: (sourceId: string) => {
    return api.post(`/sources/${sourceId}/scan/deep`)
  },

  stopSync: (sourceId: string) => {
    return api.post(`/sources/${sourceId}/sync/stop`)
  },

  getSyncStatus: (sourceId: string) => {
    return api.get(`/sources/${sourceId}/sync/status`)
  },

  createSyncProgressWebSocket: (sourceId: string) => {
    return new SyncProgressWebSocket(sourceId);
  },
}

/** Alias of `sourcesService`; the Home lanes use `sourceService.getArrivals`. */
export const sourceService = sourcesService
