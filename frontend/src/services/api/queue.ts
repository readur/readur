import api from './client'
import type { OcrActionResponse, OcrStatusResponse, QueueStats } from './types'

export const queueService = {
  getStats: () => {
    return api.get<QueueStats>('/queue/stats')
  },

  requeueFailed: () => {
    return api.post('/queue/requeue/failed')
  },

  getOcrStatus: () => {
    return api.get<OcrStatusResponse>('/queue/status')
  },

  pauseOcr: () => {
    return api.post<OcrActionResponse>('/queue/pause')
  },

  resumeOcr: () => {
    return api.post<OcrActionResponse>('/queue/resume')
  },
}
