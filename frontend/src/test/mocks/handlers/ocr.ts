/**
 * Mock handlers for OCR-related API endpoints
 */

import { http, HttpResponse } from 'msw'
import { createMockRetryStats } from '../factories'
import { applyDelay, shouldFail, createMockResponse, DEFAULT_MOCK_CONFIG } from '../utils/config'
import { MockConfig } from '../api/types'

let mockConfig: MockConfig = DEFAULT_MOCK_CONFIG

export const setOcrMockConfig = (config: Partial<MockConfig>) => {
  mockConfig = { ...mockConfig, ...config }
}

export const resetOcrState = () => {
  mockConfig = DEFAULT_MOCK_CONFIG
}

export const ocrHandlers = [
  http.get('/api/ocr/languages', async () => {
    await applyDelay(mockConfig)
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(createMockResponse(null, mockConfig), { status: mockConfig.errorCode })
    }
    return HttpResponse.json(createMockResponse({
      available_languages: [
        { code: 'eng', name: 'English', installed: true },
        { code: 'spa', name: 'Spanish', installed: true },
        { code: 'fra', name: 'French', installed: false },
      ],
      current_user_language: 'eng',
    }, mockConfig))
  }),

  http.get('/api/ocr/health', async () => {
    await applyDelay(mockConfig)
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(createMockResponse(null, mockConfig), { status: mockConfig.errorCode })
    }
    return HttpResponse.json(createMockResponse({
      status: 'healthy',
      version: '5.3.0',
      languages_available: 15,
    }, mockConfig))
  }),

  http.get('/api/documents/ocr/retry-stats', async () => {
    await applyDelay(mockConfig)
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(createMockResponse(null, mockConfig), { status: mockConfig.errorCode })
    }
    return HttpResponse.json(createMockResponse(createMockRetryStats(), mockConfig))
  }),
]