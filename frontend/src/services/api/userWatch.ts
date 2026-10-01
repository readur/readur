import api from './client'
import type {
  UserWatchDirectoryOperationResponse,
  UserWatchDirectoryResponse,
} from '../../types/generated'

export type {
  UserWatchDirectoryOperationResponse,
  UserWatchDirectoryResponse,
} from '../../types/generated'

// User Watch Directory Types
export const userWatchService = {
  getUserWatchDirectory: (userId: string) => {
    return api.get<UserWatchDirectoryResponse>(`/users/${userId}/watch/directory`)
  },

  createUserWatchDirectory: (userId: string) => {
    return api.post<UserWatchDirectoryOperationResponse>(`/users/${userId}/watch/directory`)
  },

  deleteUserWatchDirectory: (userId: string) => {
    return api.delete<UserWatchDirectoryOperationResponse>(`/users/${userId}/watch/directory`)
  },
}
