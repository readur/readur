import api from './client'
import type {
  ApiKeyResponse as ApiKey,
  CreateApiKeyRequest,
  CreateApiKeyResponse,
} from '../../types/generated'

export type {
  ApiKeyResponse as ApiKey,
  CreateApiKeyRequest,
  CreateApiKeyResponse,
} from '../../types/generated'

// ─── API Keys Service ────────────────────────────────────────────────────

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
