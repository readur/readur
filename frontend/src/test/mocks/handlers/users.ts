/**
 * Mock handlers for user-related API endpoints
 */

import { http, HttpResponse } from 'msw'
import { createMockUsers, getDefaultTestUser } from '../factories'
import { applyDelay, shouldFail, createMockResponse, DEFAULT_MOCK_CONFIG, createErrorConfig } from '../utils/config'
import { MockConfig, MockUser } from '../api/types'

let mockUsers: MockUser[] = createMockUsers(10)
let mockConfig: MockConfig = DEFAULT_MOCK_CONFIG

export const setUserMockConfig = (config: Partial<MockConfig>) => {
  mockConfig = { ...mockConfig, ...config }
}

export const setMockUsers = (users: MockUser[]) => {
  mockUsers = users
}

export const resetUsersState = () => {
  mockUsers = createMockUsers(10)
  mockConfig = DEFAULT_MOCK_CONFIG
}

export const userHandlers = [
  http.get('/api/users', async () => {
    await applyDelay(mockConfig)
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(createMockResponse(null, mockConfig), { status: mockConfig.errorCode })
    }
    return HttpResponse.json(createMockResponse(mockUsers, mockConfig))
  }),

  http.get('/api/users/:id', async ({ params }) => {
    await applyDelay(mockConfig)
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(createMockResponse(null, mockConfig), { status: mockConfig.errorCode })
    }
    const user = mockUsers.find(u => u.id === params.id)
    if (!user) {
      return HttpResponse.json(createMockResponse(null, createErrorConfig('NOT_FOUND')), { status: 404 })
    }
    return HttpResponse.json(createMockResponse(user, mockConfig))
  }),

  http.get('/api/users/:id/watch-directory', async ({ params }) => {
    await applyDelay(mockConfig)
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(createMockResponse(null, mockConfig), { status: mockConfig.errorCode })
    }
    const user = mockUsers.find(u => u.id === params.id)
    if (!user) {
      return HttpResponse.json(createMockResponse(null, createErrorConfig('NOT_FOUND')), { status: 404 })
    }
    return HttpResponse.json(createMockResponse({
      user_id: user.id,
      username: user.username,
      watch_directory_path: `/watch/${user.username}`,
      exists: true,
      enabled: true,
    }, mockConfig))
  }),
]