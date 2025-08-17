/**
 * Mock handlers for document-related API endpoints
 */

import { http, HttpResponse } from 'msw'
import { 
  createMockDocument, 
  createMockDocuments,
  createMockDocumentWithScenario,
  createMockOcrResponse,
} from '../factories'
import { 
  applyDelay, 
  shouldFail, 
  createMockResponse, 
  DEFAULT_MOCK_CONFIG,
  createErrorConfig,
} from '../utils/config'
import { MockConfig } from '../api/types'

// Mock state for documents
let mockDocuments = createMockDocuments(50)
let mockConfig: MockConfig = DEFAULT_MOCK_CONFIG

/**
 * Update mock configuration for all document handlers
 */
export const setDocumentMockConfig = (config: Partial<MockConfig>) => {
  mockConfig = { ...mockConfig, ...config }
}

/**
 * Set mock documents data
 */
export const setMockDocuments = (documents: any[]) => {
  mockDocuments = documents
}

/**
 * Reset mock documents to default state
 */
export const resetMockDocuments = () => {
  mockDocuments = createMockDocuments(50)
  mockConfig = DEFAULT_MOCK_CONFIG
}

export const documentHandlers = [
  // GET /api/documents - List documents with pagination
  http.get('/api/documents', async ({ request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const url = new URL(request.url)
    const limit = parseInt(url.searchParams.get('limit') || '20')
    const offset = parseInt(url.searchParams.get('offset') || '0')
    const ocrStatus = url.searchParams.get('ocr_status')

    let filteredDocuments = mockDocuments
    if (ocrStatus) {
      filteredDocuments = mockDocuments.filter(doc => doc.ocr_status === ocrStatus)
    }

    const paginatedDocuments = filteredDocuments.slice(offset, offset + limit)
    const total = filteredDocuments.length
    const hasMore = offset + limit < total

    const response = {
      documents: paginatedDocuments,
      pagination: {
        total,
        limit,
        offset,
        has_more: hasMore,
      },
    }

    return HttpResponse.json(createMockResponse(response, mockConfig))
  }),

  // POST /api/documents - Upload document
  http.post('/api/documents', async ({ request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const formData = await request.formData()
    const file = formData.get('file') as File
    const languages = []
    
    // Extract OCR languages from form data
    for (const [key, value] of formData.entries()) {
      if (key.startsWith('ocr_languages[')) {
        languages.push(value as string)
      }
    }

    const newDocument = createMockDocument({
      filename: file?.name || 'uploaded-file.pdf',
      file_size: file?.size || 1024000,
      mime_type: file?.type || 'application/pdf',
      source_type: 'upload',
      ocr_status: languages.length > 0 ? 'pending' : 'completed',
    })

    mockDocuments.unshift(newDocument)

    return HttpResponse.json(createMockResponse(newDocument, mockConfig))
  }),

  // GET /api/documents/:id - Get document by ID
  http.get('/api/documents/:id', async ({ params }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const document = mockDocuments.find(doc => doc.id === params.id)
    
    if (!document) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    return HttpResponse.json(createMockResponse(document, mockConfig))
  }),

  // DELETE /api/documents/:id - Delete document
  http.delete('/api/documents/:id', async ({ params }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const index = mockDocuments.findIndex(doc => doc.id === params.id)
    
    if (index === -1) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    mockDocuments.splice(index, 1)

    return HttpResponse.json(createMockResponse({ success: true }, mockConfig))
  }),

  // POST /api/documents/bulk/delete - Bulk delete documents
  http.post('/api/documents/bulk/delete', async ({ request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const body = await request.json() as { document_ids: string[] }
    const deletedCount = body.document_ids.length

    // Remove documents from mock state
    mockDocuments = mockDocuments.filter(doc => !body.document_ids.includes(doc.id))

    return HttpResponse.json(createMockResponse({ 
      success: true, 
      deleted_count: deletedCount 
    }, mockConfig))
  }),

  // GET /api/documents/:id/download - Download document
  http.get('/api/documents/:id/download', async ({ params }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const document = mockDocuments.find(doc => doc.id === params.id)
    
    if (!document) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    // Create a mock file blob
    const mockFileContent = `Mock content for ${document.filename}`
    const blob = new Blob([mockFileContent], { type: document.mime_type })

    return HttpResponse.arrayBuffer(await blob.arrayBuffer(), {
      headers: {
        'Content-Type': document.mime_type,
        'Content-Disposition': `attachment; filename="${document.filename}"`,
        'Content-Length': blob.size.toString(),
      },
    })
  }),

  // GET /api/documents/:id/thumbnail - Get document thumbnail
  http.get('/api/documents/:id/thumbnail', async ({ params }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const document = mockDocuments.find(doc => doc.id === params.id)
    
    if (!document) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    // Create a mock thumbnail blob (1x1 pixel PNG)
    const mockThumbnail = new Uint8Array([
      0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
      0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
      0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
      0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
      0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
      0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82
    ])

    return HttpResponse.arrayBuffer(mockThumbnail.buffer, {
      headers: {
        'Content-Type': 'image/png',
      },
    })
  }),

  // GET /api/documents/:id/ocr - Get OCR text
  http.get('/api/documents/:id/ocr', async ({ params }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const document = mockDocuments.find(doc => doc.id === params.id)
    
    if (!document) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    const ocrResponse = createMockOcrResponse({
      document_id: document.id,
      filename: document.filename,
      has_ocr_text: document.has_ocr_text,
      ocr_confidence: document.ocr_confidence,
      ocr_word_count: document.ocr_word_count,
      ocr_processing_time_ms: document.ocr_processing_time_ms,
      ocr_status: document.ocr_status,
    })

    return HttpResponse.json(createMockResponse(ocrResponse, mockConfig))
  }),

  // POST /api/documents/:id/ocr/retry - Retry OCR processing
  http.post('/api/documents/:id/ocr/retry', async ({ params, request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const document = mockDocuments.find(doc => doc.id === params.id)
    
    if (!document) {
      return HttpResponse.json(
        createMockResponse(null, createErrorConfig('NOT_FOUND')),
        { status: 404 }
      )
    }

    const requestData = await request.json().catch(() => ({})) as { languages?: string[] }
    
    // Update document status to pending
    document.ocr_status = 'pending'
    document.has_ocr_text = false

    return HttpResponse.json(createMockResponse({
      success: true,
      message: 'OCR retry queued successfully',
      document_id: document.id,
      languages: requestData.languages || [],
    }, mockConfig))
  }),

  // GET /api/documents/failed - Get failed documents
  http.get('/api/documents/failed', async ({ request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const url = new URL(request.url)
    const limit = parseInt(url.searchParams.get('limit') || '25')
    const offset = parseInt(url.searchParams.get('offset') || '0')
    const stage = url.searchParams.get('stage')
    const reason = url.searchParams.get('reason')

    let failedDocuments = mockDocuments.filter(doc => 
      doc.ocr_status === 'failed' || 
      (stage === 'ocr' && doc.ocr_status === 'failed')
    )

    if (reason) {
      // Filter by failure reason if provided
      failedDocuments = failedDocuments.filter(doc => 
        doc.ocr_status === 'failed'
      )
    }

    const paginatedDocuments = failedDocuments.slice(offset, offset + limit)
    const total = failedDocuments.length

    const response = {
      documents: paginatedDocuments,
      pagination: {
        total,
        limit,
        offset,
        has_more: offset + limit < total,
      },
    }

    return HttpResponse.json(createMockResponse(response, mockConfig))
  }),

  // GET /api/documents/duplicates - Get duplicate documents
  http.get('/api/documents/duplicates', async ({ request }) => {
    await applyDelay(mockConfig)
    
    if (shouldFail(mockConfig)) {
      return HttpResponse.json(
        createMockResponse(null, mockConfig),
        { status: mockConfig.errorCode }
      )
    }

    const url = new URL(request.url)
    const limit = parseInt(url.searchParams.get('limit') || '25')
    const offset = parseInt(url.searchParams.get('offset') || '0')

    // Create some mock duplicates
    const duplicateDocuments = [
      createMockDocumentWithScenario('duplicate_document'),
      createMockDocumentWithScenario('duplicate_document'),
    ]

    const response = {
      duplicates: duplicateDocuments.slice(offset, offset + limit),
      total: duplicateDocuments.length,
    }

    return HttpResponse.json(createMockResponse(response, mockConfig))
  }),
]