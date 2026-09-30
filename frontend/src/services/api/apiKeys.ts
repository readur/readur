import api from './client'

// ─── API Keys Service ────────────────────────────────────────────────────

export interface ApiKey {
  id: string
  user_id: string
  name: string
  key_prefix: string
  expires_at: string | null
  last_used_at: string | null
  revoked_at: string | null
  is_expired: boolean
  created_at: string
}

export interface CreateApiKeyRequest {
  name: string
  expires_in_days?: number | null
}

export interface CreateApiKeyResponse {
  api_key: ApiKey
  // Full plaintext token — returned ONLY from the create endpoint and must be
  // captured by the user immediately. The server has no way to recover it.
  plaintext: string
}

export const apiKeysService = {
  create: (request: CreateApiKeyRequest) => {
    return api.post<CreateApiKeyResponse>('/auth/keys', request)
  },

  list: (all = false) => {
    return api.get<ApiKey[]>('/auth/keys', { params: all ? { all: true } : {} })
  },

  revoke: (id: string) => {
    return api.delete(`/auth/keys/${id}`)
  },
}
