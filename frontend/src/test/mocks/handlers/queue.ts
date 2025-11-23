/**
 * Mock handlers for queue-related API endpoints
 */

import { http, HttpResponse } from 'msw'
import { 
  createMockQueueStats,
} from '../factories'
import { 
  applyDelay, 
  shouldFail, 
  createMockResponse, 
  DEFAULT_MOCK_CONFIG,
} from '../utils/config'
import { MockConfig, QueueStats } from '../api/types'

// Mock state for queue
let mockQueueStats: QueueStats = createMockQueueStats()
let ocrPaused = false
let mockConfig: MockConfig = DEFAULT_MOCK_CONFIG

/**
 * Update mock configuration for all queue handlers
 */
export const setQueueMockConfig = (config: Partial<MockConfig>) => {
  mockConfig = { ...mockConfig, ...config }
}

/**
 * Set mock queue statistics
 */
export const setMockQueueStats = (stats: QueueStats) => {
  mockQueueStats = stats
}

/**
 * Set OCR paused state
 */
export const setOcrPaused = (paused: boolean) => {
  ocrPaused = paused
}

/**
 * Reset queue mock state
 */
export const resetQueueState = () => {
  mockQueueStats = createMockQueueStats()
  ocrPaused = false
  mockConfig = DEFAULT_MOCK_CONFIG
}

export const queueHandlers = [
  // GET /api/queue/stats - Get queue statistics
  http.get('/api/queue/stats', async () => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    return HttpResponse.json(createMockResponse(mockQueueStats, mockConfig))
  }),

  // POST /api/queue/requeue/failed - Requeue failed items
  http.post('/api/queue/requeue/failed', async () => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const requeuedCount = mockQueueStats.failed_count
    
    // Update queue stats - move failed to pending
    mockQueueStats = {
      ...mockQueueStats,
      pending_count: mockQueueStats.pending_count + requeuedCount,
      failed_count: 0,
    }

    return HttpResponse.json(createMockResponse({
      success: true,
      message: `${requeuedCount} items requeued successfully`,
      requeued_count: requeuedCount,
    }, mockConfig))
  }),

  // GET /api/queue/status - Get OCR processing status
  http.get('/api/queue/status', async () => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    return HttpResponse.json(createMockResponse({
      is_paused: ocrPaused,
      status: ocrPaused ? 'paused' : 'running',
    }, mockConfig))
  }),

  // POST /api/queue/pause - Pause OCR processing
  http.post('/api/queue/pause', async () => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    ocrPaused = true

    return HttpResponse.json(createMockResponse({
      status: 'paused',
      message: 'OCR processing paused successfully',
    }, mockConfig))
  }),

  // POST /api/queue/resume - Resume OCR processing
  http.post('/api/queue/resume', async () => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    ocrPaused = false

    return HttpResponse.json(createMockResponse({
      status: 'resumed',
      message: 'OCR processing resumed successfully',
    }, mockConfig))
  }),

  // GET /api/queue/health - Get queue health status
  http.get('/api/queue/health', async () => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const isHealthy = mockQueueStats.processing_count < 10 && mockQueueStats.failed_count < 50
    
    return HttpResponse.json(createMockResponse({
      healthy: isHealthy,
      status: isHealthy ? 'ok' : 'degraded',
      details: {
        pending_queue_size: mockQueueStats.pending_count,
        processing_queue_size: mockQueueStats.processing_count,
        failed_queue_size: mockQueueStats.failed_count,
        is_paused: ocrPaused,
      },
      timestamp: new Date().toISOString(),
    }, mockConfig))
  }),

  // POST /api/queue/clear - Clear specific queue
  http.post('/api/queue/clear', async ({ request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const { queue_type } = await request.json()
    let clearedCount = 0

    switch (queue_type) {
      case 'failed':
        clearedCount = mockQueueStats.failed_count
        mockQueueStats.failed_count = 0
        break
      case 'pending':
        clearedCount = mockQueueStats.pending_count
        mockQueueStats.pending_count = 0
        break
      case 'all':
        clearedCount = mockQueueStats.pending_count + mockQueueStats.failed_count
        mockQueueStats.pending_count = 0
        mockQueueStats.failed_count = 0
        break
      default:
        return HttpResponse.json(
          createMockResponse(null, { 
            ...mockConfig, 
            shouldFail: true, 
            errorCode: 400, 
            errorMessage: 'Invalid queue type' 
          }),
          { status: 400 }
        )
    }

    return HttpResponse.json(createMockResponse({
      success: true,
      message: `${clearedCount} items cleared from ${queue_type} queue`,
      cleared_count: clearedCount,
    }, mockConfig))
  }),

  // GET /api/queue/metrics - Get detailed queue metrics
  http.get('/api/queue/metrics', async () => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    return HttpResponse.json(createMockResponse({
      current_stats: mockQueueStats,
      hourly_stats: [
        { hour: new Date().getHours() - 1, processed: 45, failed: 2 },
        { hour: new Date().getHours(), processed: 32, failed: 1 },
      ],
      daily_stats: [
        { date: new Date().toISOString().split('T')[0], processed: 234, failed: 12 },
      ],
      processing_rates: {
        files_per_minute: 5.2,
        avg_processing_time_ms: 2500,
        success_rate: 0.95,
      },
    }, mockConfig))
  }),
]