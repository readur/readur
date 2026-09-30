import api from './client'
import { SyncProgressWebSocket } from './syncProgress'

export const sourcesService = {
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
