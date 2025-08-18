/**
 * Mock handlers for label-related API endpoints
 */

import { http, HttpResponse } from 'msw'
import { 
  createMockLabel,
  createDefaultLabels,
} from '../factories'
import { 
  applyDelay, 
  shouldFail, 
  createMockResponse, 
  DEFAULT_MOCK_CONFIG,
  createErrorConfig,
} from '../utils/config'
import { MockConfig, MockLabel } from '../api/types'

// Mock state for labels
let mockLabels: MockLabel[] = createDefaultLabels()
let mockConfig: MockConfig = DEFAULT_MOCK_CONFIG

/**
 * Update mock configuration for all label handlers
 */
export const setLabelMockConfig = (config: Partial<MockConfig>) => {
  mockConfig = { ...mockConfig, ...config }
}

/**
 * Set mock labels data
 */
export const setMockLabels = (labels: MockLabel[]) => {
  mockLabels = labels
}

/**
 * Reset labels mock state
 */
export const resetLabelsState = () => {
  mockLabels = createDefaultLabels()
  mockConfig = DEFAULT_MOCK_CONFIG
}

export const labelHandlers = [
  // GET /api/labels - List labels
  http.get('/api/labels', async () => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    return HttpResponse.json(createMockResponse(mockLabels, mockConfig))
  }),

  // POST /api/labels - Create label
  http.post('/api/labels', async ({ request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const labelData = await request.json() as { name: string; description?: string; color?: string }
    
    // Check for duplicate name
    const existingLabel = mockLabels.find(l => l.name === labelData.name)
    if (existingLabel) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('CONFLICT', 'Label with this name already exists')),
        { status: 409 }
      )
    }

    const newLabel = createMockLabel({
      ...labelData,
      document_count: 0,
    })

    mockLabels.push(newLabel)

    return HttpResponse.json(createMockResponse(newLabel, mockConfig))
  }),

  // GET /api/labels/:id - Get label by ID
  http.get('/api/labels/:id', async ({ params }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const label = mockLabels.find(l => l.id === params.id)
    
    if (!label) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    return HttpResponse.json(createMockResponse(label, mockConfig))
  }),

  // PUT /api/labels/:id - Update label
  http.put('/api/labels/:id', async ({ params, request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const labelIndex = mockLabels.findIndex(l => l.id === params.id)
    
    if (labelIndex === -1) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    const updateData = await request.json() as Partial<{ name: string; description: string; color: string }>
    
    // Check for duplicate name if name is being changed
    if (updateData.name && updateData.name !== mockLabels[labelIndex].name) {
      const existingLabel = mockLabels.find(l => l.name === updateData.name)
      if (existingLabel) {
        return HttpResponse.json(
          createMockResponse(null, createErrorConfig('CONFLICT', 'Label with this name already exists')),
          { status: 409 }
        )
      }
    }

    const updatedLabel = {
      ...mockLabels[labelIndex],
      ...updateData,
      updated_at: new Date().toISOString(),
    }

    mockLabels[labelIndex] = updatedLabel

    return HttpResponse.json(createMockResponse(updatedLabel, mockConfig))
  }),

  // DELETE /api/labels/:id - Delete label
  http.delete('/api/labels/:id', async ({ params }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const labelIndex = mockLabels.findIndex(l => l.id === params.id)
    
    if (labelIndex === -1) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    const label = mockLabels[labelIndex]
    mockLabels.splice(labelIndex, 1)

    return HttpResponse.json(createMockResponse({
      success: true,
      message: 'Label deleted successfully',
      documents_affected: label.document_count || 0,
    }, mockConfig))
  }),

  // GET /api/labels/:id/documents - Get documents with this label
  http.get('/api/labels/:id/documents', async ({ params, request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const label = mockLabels.find(l => l.id === params.id)
    
    if (!label) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    const url = new URL(request.url)
    const limit = parseInt(url.searchParams.get('limit') || '20')
    const offset = parseInt(url.searchParams.get('offset') || '0')

    // Mock documents with this label
    const mockDocuments = Array.from({ length: Math.min(limit, label.document_count || 0) }, (_, i) => ({
      id: `doc-${label.id}-${offset + i}`,
      filename: `document-${offset + i}.pdf`,
      tags: [label.name],
    }))

    return HttpResponse.json(createMockResponse({
      documents: mockDocuments,
      pagination: {
        total: label.document_count || 0,
        limit,
        offset,
        has_more: offset + limit < (label.document_count || 0),
      },
    }, mockConfig))
  }),

  // POST /api/labels/:id/documents - Add documents to label
  http.post('/api/labels/:id/documents', async ({ params, request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const label = mockLabels.find(l => l.id === params.id)
    
    if (!label) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    const body = await request.json() as { document_ids: string[] }
    
    // Update label document count
    label.document_count = (label.document_count || 0) + body.document_ids.length
    label.updated_at = new Date().toISOString()

    return HttpResponse.json(createMockResponse({
      success: true,
      message: `${body.document_ids.length} documents added to label`,
      label_id: label.id,
      documents_added: body.document_ids.length,
    }, mockConfig))
  }),

  // DELETE /api/labels/:id/documents - Remove documents from label
  http.delete('/api/labels/:id/documents', async ({ params, request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const label = mockLabels.find(l => l.id === params.id)
    
    if (!label) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    const body = await request.json() as { document_ids: string[] }
    
    // Update label document count
    label.document_count = Math.max(0, (label.document_count || 0) - body.document_ids.length)
    label.updated_at = new Date().toISOString()

    return HttpResponse.json(createMockResponse({
      success: true,
      message: `${body.document_ids.length} documents removed from label`,
      label_id: label.id,
      documents_removed: body.document_ids.length,
    }, mockConfig))
  }),

  // GET /api/labels/stats - Get label statistics
  http.get('/api/labels/stats', async () => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const totalLabels = mockLabels.length
    const totalDocumentsLabeled = mockLabels.reduce((sum, label) => sum + (label.document_count || 0), 0)
    const mostUsedLabel = mockLabels.reduce((max, label) => 
      (label.document_count || 0) > (max.document_count || 0) ? label : max
    )

    return HttpResponse.json(createMockResponse({
      total_labels: totalLabels,
      total_documents_labeled: totalDocumentsLabeled,
      most_used_label: {
        id: mostUsedLabel.id,
        name: mostUsedLabel.name,
        document_count: mostUsedLabel.document_count,
      },
      labels_by_usage: mockLabels
        .map(label => ({
          id: label.id,
          name: label.name,
          document_count: label.document_count || 0,
        }))
        .sort((a, b) => b.document_count - a.document_count),
    }, mockConfig))
  }),
]