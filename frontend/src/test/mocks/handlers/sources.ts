/**
 * Mock handlers for source-related API endpoints
 */

import { http, HttpResponse } from 'msw'
import { 
  createMockSource,
  createMockSources,
  createMockSyncProgress,
  createMockSyncProgressWithScenario,
} from '../factories'
import { 
  applyDelay, 
  shouldFail, 
  createMockResponse, 
  DEFAULT_MOCK_CONFIG,
  createErrorConfig,
} from '../utils/config'
import { MockConfig, MockSource, MockSyncProgress } from '../api/types'

// Mock state for sources
let mockSources: MockSource[] = createMockSources(5)
let syncProgress: Map<string, MockSyncProgress> = new Map()
let mockConfig: MockConfig = DEFAULT_MOCK_CONFIG

/**
 * Update mock configuration for all source handlers
 */
export const setSourceMockConfig = (config: Partial<MockConfig>) => {
  mockConfig = { ...mockConfig, ...config }
}

/**
 * Set mock sources data
 */
export const setMockSources = (sources: MockSource[]) => {
  mockSources = sources
}

/**
 * Set sync progress for a source
 */
export const setSyncProgress = (sourceId: string, progress: MockSyncProgress) => {
  syncProgress.set(sourceId, progress)
}

/**
 * Reset sources mock state
 */
export const resetSourcesState = () => {
  mockSources = createMockSources(5)
  syncProgress.clear()
  mockConfig = DEFAULT_MOCK_CONFIG
}

export const sourceHandlers = [
  // GET /api/sources - List sources
  http.get('/api/sources', async () => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    return HttpResponse.json(createMockResponse(mockSources, mockConfig))
  }),

  // POST /api/sources - Create source
  http.post('/api/sources', async ({ request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const sourceData = await request.json()
    
    const newSource = createMockSource({
      ...sourceData,
      enabled: true,
      sync_status: 'idle',
      last_sync_at: undefined,
    })

    mockSources.push(newSource)

    return HttpResponse.json(createMockResponse(newSource, mockConfig))
  }),

  // GET /api/sources/:id - Get source by ID
  http.get('/api/sources/:id', async ({ params }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const source = mockSources.find(s => s.id === params.id)
    
    if (!source) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    return HttpResponse.json(createMockResponse(source, mockConfig))
  }),

  // PUT /api/sources/:id - Update source
  http.put('/api/sources/:id', async ({ params, request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const sourceIndex = mockSources.findIndex(s => s.id === params.id)
    
    if (sourceIndex === -1) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    const updateData = await request.json()
    const updatedSource = {
      ...mockSources[sourceIndex],
      ...updateData,
      updated_at: new Date().toISOString(),
    }

    mockSources[sourceIndex] = updatedSource

    return HttpResponse.json(createMockResponse(updatedSource, mockConfig))
  }),

  // DELETE /api/sources/:id - Delete source
  http.delete('/api/sources/:id', async ({ params }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const sourceIndex = mockSources.findIndex(s => s.id === params.id)
    
    if (sourceIndex === -1) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    mockSources.splice(sourceIndex, 1)
    syncProgress.delete(params.id as string)

    return HttpResponse.json(createMockResponse({
      success: true,
      message: 'Source deleted successfully',
    }, mockConfig))
  }),

  // POST /api/sources/:id/sync - Trigger sync
  http.post('/api/sources/:id/sync', async ({ params }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const source = mockSources.find(s => s.id === params.id)
    
    if (!source) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    if (!source.enabled) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('BAD_REQUEST', 'Source is disabled')),
        { status: 400 }
      )
    }

    // Update source status
    source.sync_status = 'syncing'
    
    // Create initial sync progress
    const progress = createMockSyncProgressWithScenario('just_started')
    progress.source_id = source.id
    syncProgress.set(source.id, progress)

    return HttpResponse.json(createMockResponse({
      success: true,
      message: 'Sync started successfully',
      source_id: source.id,
      sync_id: `sync-${Date.now()}`,
    }, mockConfig))
  }),

  // POST /api/sources/:id/deep-scan - Trigger deep scan
  http.post('/api/sources/:id/deep-scan', async ({ params }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const source = mockSources.find(s => s.id === params.id)
    
    if (!source) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    // Update source status
    source.sync_status = 'syncing'
    
    // Create deep scan progress
    const progress = createMockSyncProgressWithScenario('just_started')
    progress.source_id = source.id
    progress.phase_description = 'Starting deep scan...'
    syncProgress.set(source.id, progress)

    return HttpResponse.json(createMockResponse({
      success: true,
      message: 'Deep scan started successfully',
      source_id: source.id,
      scan_type: 'deep',
    }, mockConfig))
  }),

  // POST /api/sources/:id/sync/stop - Stop sync
  http.post('/api/sources/:id/sync/stop', async ({ params }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const source = mockSources.find(s => s.id === params.id)
    
    if (!source) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    // Update source status
    source.sync_status = 'idle'
    source.last_sync_at = new Date().toISOString()
    
    // Remove sync progress
    syncProgress.delete(source.id)

    return HttpResponse.json(createMockResponse({
      success: true,
      message: 'Sync stopped successfully',
      source_id: source.id,
    }, mockConfig))
  }),

  // GET /api/sources/:id/sync/status - Get sync status
  http.get('/api/sources/:id/sync/status', async ({ params }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const source = mockSources.find(s => s.id === params.id)
    
    if (!source) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    const progress = syncProgress.get(source.id)

    return HttpResponse.json(createMockResponse({
      source_id: source.id,
      sync_status: source.sync_status,
      last_sync_at: source.last_sync_at,
      progress: progress || null,
      is_syncing: source.sync_status === 'syncing',
    }, mockConfig))
  }),

  // GET /api/sources/:id/sync/progress - Get detailed sync progress
  http.get('/api/sources/:id/sync/progress', async ({ params }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const source = mockSources.find(s => s.id === params.id)
    
    if (!source) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    const progress = syncProgress.get(source.id) || createMockSyncProgressWithScenario('idle')
    progress.source_id = source.id

    return HttpResponse.json(createMockResponse(progress, mockConfig))
  }),

  // POST /api/sources/:id/test - Test source connection
  http.post('/api/sources/:id/test', async ({ params }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const source = mockSources.find(s => s.id === params.id)
    
    if (!source) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    const isConnectionSuccessful = source.source_type !== 's3' || !source.path.includes('error')

    if (!isConnectionSuccessful) {
      return HttpResponse.json(createMockResponse({
        success: false,
        message: 'Connection failed: Invalid credentials or unreachable endpoint',
        details: {
          error_code: 'CONNECTION_FAILED',
          tested_at: new Date().toISOString(),
        },
      }, mockConfig))
    }

    return HttpResponse.json(createMockResponse({
      success: true,
      message: 'Connection test successful',
      details: {
        source_type: source.source_type,
        path_accessible: true,
        tested_at: new Date().toISOString(),
      },
    }, mockConfig))
  }),

  // GET /api/sources/:id/stats - Get source statistics
  http.get('/api/sources/:id/stats', async ({ params }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const source = mockSources.find(s => s.id === params.id)
    
    if (!source) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    return HttpResponse.json(createMockResponse({
      source_id: source.id,
      total_documents: Math.floor(Math.random() * 1000) + 100,
      total_size_bytes: Math.floor(Math.random() * 1000000000) + 100000000,
      last_sync_duration_ms: Math.floor(Math.random() * 300000) + 30000,
      sync_frequency: '6h',
      success_rate: 0.95,
      error_count: Math.floor(Math.random() * 5),
      warning_count: Math.floor(Math.random() * 10),
    }, mockConfig))
  }),
]