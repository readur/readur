import axios from 'axios'
import api from './client'
import type {
  CreateSharedLinkRequest,
  SharedDocumentMetadata,
  SharedLinkResponse as SharedLinkData,
} from '../../types/generated'

export type {
  CreateSharedLinkRequest,
  SharedDocumentMetadata,
  SharedLinkResponse as SharedLinkData,
} from '../../types/generated'

export const sharedLinksService = {
  create: (request: CreateSharedLinkRequest) => {
    return api.post<SharedLinkData>('/shared/links', request)
  },

  listAll: () => {
    return api.get<SharedLinkData[]>('/shared/links')
  },

  listByDocument: (documentId: string) => {
    return api.get<SharedLinkData[]>(`/shared/links/document/${documentId}`)
  },

  revoke: (linkId: string) => {
    return api.delete(`/shared/links/${linkId}`)
  },
}

// Public (unauthenticated) shared link access — uses a separate axios instance
const publicApi = axios.create({
  baseURL: '/api',
  headers: { 'Content-Type': 'application/json' },
})

export const sharedLinksPublicService = {
  getMetadata: (token: string) => {
    return publicApi.get<SharedDocumentMetadata>(`/public/shared/${token}`)
  },

  verifyPassword: (token: string, password: string) => {
    return publicApi.post<{ valid: boolean }>(`/public/shared/${token}/verify`, { password })
  },

  downloadDocument: async (token: string, password?: string) => {
    const response = await publicApi.post(`/public/shared/${token}/download`, { password: password || null }, {
      responseType: 'blob',
    })
    return response
  },

  viewDocument: async (token: string, password?: string) => {
    const response = await publicApi.post(`/public/shared/${token}/view`, { password: password || null }, {
      responseType: 'blob',
    })
    return response
  },
}
