import axios from 'axios'
import api from './client'

export interface SharedLinkData {
  id: string
  document_id: string
  token: string
  url: string
  has_password: boolean
  expires_at: string | null
  max_views: number | null
  view_count: number
  is_expired: boolean
  is_revoked: boolean
  created_at: string
}

export interface CreateSharedLinkRequest {
  document_id: string
  password?: string
  expires_at?: string
  max_views?: number
}

export interface SharedDocumentMetadata {
  filename: string
  original_filename: string
  file_size: number
  mime_type: string
  requires_password: boolean
  created_at: string
}

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
