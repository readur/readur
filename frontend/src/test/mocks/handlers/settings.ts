/**
 * Mock handlers for settings-related API endpoints
 */

import { http, HttpResponse } from 'msw'
import { applyDelay, shouldFail, createMockResponse, DEFAULT_MOCK_CONFIG } from '../utils/config'
import { MockConfig } from '../api/types'

let mockSettings = {
  ocr_enabled: true,
  max_file_size_mb: 100,
  allowed_file_types: ['pdf', 'jpg', 'png', 'txt'],
  auto_ocr: true,
  notification_enabled: true,
}
let mockConfig: MockConfig = DEFAULT_MOCK_CONFIG

export const setSettingsMockConfig = (config: Partial<MockConfig>) => {
  mockConfig = { ...mockConfig, ...config }
}

export const setMockSettings = (settings: any) => {
  mockSettings = { ...mockSettings, ...settings }
}

export const resetSettingsState = () => {
  mockSettings = {
    ocr_enabled: true,
    max_file_size_mb: 100,
    allowed_file_types: ['pdf', 'jpg', 'png', 'txt'],
    auto_ocr: true,
    notification_enabled: true,
  }
  mockConfig = DEFAULT_MOCK_CONFIG
}

export const settingsHandlers = [
  http.get('/api/settings', async () => {
    await applyDelay(mockConfig)
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(createMockResponse(null, mockConfig), { status: mockConfig.errorCode })
    }
    return HttpResponse.json(createMockResponse(mockSettings, mockConfig))
  }),

  http.put('/api/settings', async ({ request }) => {
    await applyDelay(mockConfig)
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(createMockResponse(null, mockConfig), { status: mockConfig.errorCode })
    }
    const updates = await request.json()
    mockSettings = { ...mockSettings, ...updates }
    return HttpResponse.json(createMockResponse(mockSettings, mockConfig))
  }),
]