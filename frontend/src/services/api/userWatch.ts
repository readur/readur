import api from './client'

// User Watch Directory Types
export interface UserWatchDirectoryResponse {
  user_id: string;
  username: string;
  watch_directory_path: string;
  exists: boolean;
  enabled: boolean;
}

export interface UserWatchDirectoryOperationResponse {
  success: boolean;
  message: string;
  watch_directory_path?: string;
}

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
