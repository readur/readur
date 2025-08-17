/**
 * useMockDocuments - Document CRUD operations with mock data
 * Provides comprehensive document management for testing scenarios
 */

import { useState, useCallback, useEffect, useRef } from 'react'
import { useMockApiContext } from '../providers/MockApiProvider'
import { createMockDocument, createMockDocumentWithScenario, createMockDocuments } from '../../factories/document'
import type { MockDocument } from '../../api/types'

export interface DocumentFilter {
  search?: string
  type?: string
  status?: string
  dateRange?: { start: Date; end: Date }
  labels?: string[]
  minSize?: number
  maxSize?: number
  hasOcr?: boolean
  user?: string
}

export interface DocumentSort {
  field: keyof MockDocument
  direction: 'asc' | 'desc'
}

export interface PaginationOptions {
  page: number
  limit: number
}

export interface UseMockDocumentsReturn {
  // Document state
  documents: MockDocument[]
  filteredDocuments: MockDocument[]
  totalCount: number
  isLoading: boolean
  
  // CRUD operations
  addDocument: (document?: Partial<MockDocument>) => Promise<MockDocument>
  removeDocument: (id: string) => Promise<void>
  updateDocument: (id: string, updates: Partial<MockDocument>) => Promise<MockDocument | null>
  getDocument: (id: string) => MockDocument | null
  
  // Bulk operations
  addMultipleDocuments: (count: number, overrides?: Partial<MockDocument>) => Promise<MockDocument[]>
  removeMultipleDocuments: (ids: string[]) => Promise<void>
  bulkUpdateDocuments: (ids: string[], updates: Partial<MockDocument>) => Promise<void>
  clearAllDocuments: () => void
  
  // Filtering and searching
  setFilter: (filter: Partial<DocumentFilter>) => void
  clearFilter: () => void
  setSort: (sort: DocumentSort) => void
  setPagination: (options: PaginationOptions) => void
  
  // Document scenarios
  simulateUploadProcess: (filename: string) => Promise<MockDocument>
  simulateOcrProcess: (documentId: string) => Promise<void>
  simulateFailedOcr: (documentId: string) => Promise<void>
  simulateDuplicateDetection: () => MockDocument[]
  
  // Document operations
  simulateDocumentView: (id: string) => void
  simulateDocumentDownload: (id: string) => Promise<Blob>
  simulateDocumentShare: (id: string, shareType: 'link' | 'email') => Promise<string>
  
  // Testing utilities
  generateTestDocuments: (scenario: DocumentTestScenario) => Promise<MockDocument[]>
  resetToScenario: (scenario: string) => void
  exportDocuments: (format: 'json' | 'csv') => string
  importDocuments: (data: string, format: 'json' | 'csv') => Promise<MockDocument[]>
  
  // Performance testing
  stressTestDocuments: (count: number) => Promise<void>
  measureOperationTime: <T>(operation: () => Promise<T>) => Promise<{ result: T; duration: number }>
  
  // Statistics
  getDocumentStats: () => DocumentStatistics
  getUsageMetrics: () => UsageMetrics
}

export interface DocumentTestScenario {
  name: string
  count: number
  types?: string[]
  withOcr?: boolean
  withErrors?: boolean
  recentUploads?: boolean
}

export interface DocumentStatistics {
  total: number
  byType: Record<string, number>
  byStatus: Record<string, number>
  byUser: Record<string, number>
  averageSize: number
  totalSize: number
  ocrSuccessRate: number
  uploadTrend: Array<{ date: string; count: number }>
}

export interface UsageMetrics {
  viewsPerDocument: Record<string, number>
  downloadsPerDocument: Record<string, number>
  searchHits: Record<string, number>
  popularDocuments: MockDocument[]
  recentActivity: Array<{ action: string; documentId: string; timestamp: Date }>
}

const DOCUMENT_TEST_SCENARIOS: Record<string, DocumentTestScenario> = {
  empty: { name: 'Empty State', count: 0 },
  minimal: { name: 'Minimal Documents', count: 3 },
  typical: { name: 'Typical Usage', count: 25, withOcr: true },
  heavy: { name: 'Heavy Usage', count: 100, withOcr: true },
  mixed_types: { name: 'Mixed File Types', count: 20, types: ['pdf', 'image', 'text', 'office'] },
  with_errors: { name: 'With Errors', count: 15, withErrors: true },
  recent_uploads: { name: 'Recent Uploads', count: 10, recentUploads: true },
}

export const useMockDocuments = (): UseMockDocumentsReturn => {
  const [documents, setDocuments] = useState<MockDocument[]>([])
  const [filter, setFilterState] = useState<DocumentFilter>({})
  const [sort, setSortState] = useState<DocumentSort>({ field: 'created_at', direction: 'desc' })
  const [pagination, setPaginationState] = useState<PaginationOptions>({ page: 1, limit: 20 })
  const [isLoading, setIsLoading] = useState(false)
  const [usageMetrics, setUsageMetrics] = useState<UsageMetrics>({
    viewsPerDocument: {},
    downloadsPerDocument: {},
    searchHits: {},
    popularDocuments: [],
    recentActivity: [],
  })
  
  const apiContext = useMockApiContext?.()
  const operationTimeRef = useRef<{ start: number; operation: string } | null>(null)

  // Apply filtering and sorting
  const filteredDocuments = documents
    .filter(doc => applyDocumentFilter(doc, filter))
    .sort((a, b) => applySorting(a, b, sort))

  const totalCount = filteredDocuments.length
  const paginatedDocuments = filteredDocuments.slice(
    (pagination.page - 1) * pagination.limit,
    pagination.page * pagination.limit
  )

  // CRUD operations
  const addDocument = useCallback(async (docData: Partial<MockDocument> = {}): Promise<MockDocument> => {
    setIsLoading(true)
    
    // Simulate API delay
    await new Promise(resolve => setTimeout(resolve, 100))
    
    const newDocument = createMockDocument({
      ...docData,
      id: docData.id || `doc_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
    })
    
    setDocuments(prev => [newDocument, ...prev])
    recordActivity('add', newDocument.id)
    setIsLoading(false)
    
    return newDocument
  }, [])

  const removeDocument = useCallback(async (id: string): Promise<void> => {
    setIsLoading(true)
    await new Promise(resolve => setTimeout(resolve, 50))
    
    setDocuments(prev => prev.filter(doc => doc.id !== id))
    recordActivity('remove', id)
    setIsLoading(false)
  }, [])

  const updateDocument = useCallback(async (id: string, updates: Partial<MockDocument>): Promise<MockDocument | null> => {
    setIsLoading(true)
    await new Promise(resolve => setTimeout(resolve, 75))
    
    let updatedDocument: MockDocument | null = null
    setDocuments(prev => prev.map(doc => {
      if (doc.id === id) {
        updatedDocument = { ...doc, ...updates, updated_at: new Date().toISOString() }
        return updatedDocument
      }
      return doc
    }))
    
    if (updatedDocument) {
      recordActivity('update', id)
    }
    
    setIsLoading(false)
    return updatedDocument
  }, [])

  const getDocument = useCallback((id: string): MockDocument | null => {
    const doc = documents.find(d => d.id === id)
    if (doc) {
      recordActivity('view', id)
    }
    return doc || null
  }, [documents])

  // Bulk operations
  const addMultipleDocuments = useCallback(async (count: number, overrides: Partial<MockDocument> = {}): Promise<MockDocument[]> => {
    setIsLoading(true)
    
    const newDocuments = createMockDocuments(count, { overrides })
    
    // Simulate processing time based on count
    await new Promise(resolve => setTimeout(resolve, Math.min(count * 10, 1000)))
    
    setDocuments(prev => [...newDocuments, ...prev])
    setIsLoading(false)
    
    return newDocuments
  }, [])

  const removeMultipleDocuments = useCallback(async (ids: string[]): Promise<void> => {
    setIsLoading(true)
    await new Promise(resolve => setTimeout(resolve, ids.length * 20))
    
    setDocuments(prev => prev.filter(doc => !ids.includes(doc.id)))
    ids.forEach(id => recordActivity('remove', id))
    setIsLoading(false)
  }, [])

  const bulkUpdateDocuments = useCallback(async (ids: string[], updates: Partial<MockDocument>): Promise<void> => {
    setIsLoading(true)
    await new Promise(resolve => setTimeout(resolve, ids.length * 30))
    
    setDocuments(prev => prev.map(doc => {
      if (ids.includes(doc.id)) {
        recordActivity('update', doc.id)
        return { ...doc, ...updates, updated_at: new Date().toISOString() }
      }
      return doc
    }))
    
    setIsLoading(false)
  }, [])

  const clearAllDocuments = useCallback(() => {
    setDocuments([])
    recordActivity('clear_all', 'all')
  }, [])

  // Filtering and searching
  const setFilter = useCallback((newFilter: Partial<DocumentFilter>) => {
    setFilterState(prev => ({ ...prev, ...newFilter }))
    setPaginationState(prev => ({ ...prev, page: 1 })) // Reset to first page
  }, [])

  const clearFilter = useCallback(() => {
    setFilterState({})
    setPaginationState(prev => ({ ...prev, page: 1 }))
  }, [])

  const setSort = useCallback((newSort: DocumentSort) => {
    setSortState(newSort)
    setPaginationState(prev => ({ ...prev, page: 1 }))
  }, [])

  const setPagination = useCallback((options: PaginationOptions) => {
    setPaginationState(options)
  }, [])

  // Document scenarios
  const simulateUploadProcess = useCallback(async (filename: string): Promise<MockDocument> => {
    // Stage 1: File uploaded
    const doc = await addDocument({
      filename,
      processing_status: 'uploading',
      ocr_status: 'pending',
      upload_progress: 0,
    })

    // Stage 2: Upload progress
    for (let progress = 25; progress <= 100; progress += 25) {
      await new Promise(resolve => setTimeout(resolve, 200))
      await updateDocument(doc.id, { upload_progress: progress })
    }

    // Stage 3: Processing starts
    await updateDocument(doc.id, {
      processing_status: 'processing',
      ocr_status: 'processing',
    })

    // Stage 4: OCR completion
    await new Promise(resolve => setTimeout(resolve, 1000))
    await updateDocument(doc.id, {
      processing_status: 'completed',
      ocr_status: 'completed',
      ocr_text: `This is mock OCR text for ${filename}`,
      ocr_confidence: 0.95,
    })

    return doc
  }, [addDocument, updateDocument])

  const simulateOcrProcess = useCallback(async (documentId: string): Promise<void> => {
    await updateDocument(documentId, { ocr_status: 'processing' })
    
    // Simulate processing time
    await new Promise(resolve => setTimeout(resolve, 2000))
    
    await updateDocument(documentId, {
      ocr_status: 'completed',
      ocr_text: `Mock OCR text for document ${documentId}`,
      ocr_confidence: 0.9 + Math.random() * 0.1,
    })
  }, [updateDocument])

  const simulateFailedOcr = useCallback(async (documentId: string): Promise<void> => {
    await updateDocument(documentId, { ocr_status: 'processing' })
    await new Promise(resolve => setTimeout(resolve, 1500))
    await updateDocument(documentId, {
      ocr_status: 'failed',
      error_message: 'OCR processing failed: Unable to extract text from image',
    })
  }, [updateDocument])

  const simulateDuplicateDetection = useCallback((): MockDocument[] => {
    const duplicates = documents.filter((doc, index) => 
      documents.findIndex(d => d.filename === doc.filename) !== index
    )
    return duplicates
  }, [documents])

  // Document operations
  const simulateDocumentView = useCallback((id: string) => {
    recordActivity('view', id)
    setUsageMetrics(prev => ({
      ...prev,
      viewsPerDocument: {
        ...prev.viewsPerDocument,
        [id]: (prev.viewsPerDocument[id] || 0) + 1,
      },
    }))
  }, [])

  const simulateDocumentDownload = useCallback(async (id: string): Promise<Blob> => {
    recordActivity('download', id)
    setUsageMetrics(prev => ({
      ...prev,
      downloadsPerDocument: {
        ...prev.downloadsPerDocument,
        [id]: (prev.downloadsPerDocument[id] || 0) + 1,
      },
    }))
    
    // Simulate download delay
    await new Promise(resolve => setTimeout(resolve, 300))
    
    const doc = getDocument(id)
    const content = `Mock file content for ${doc?.filename || 'unknown file'}`
    return new Blob([content], { type: 'text/plain' })
  }, [getDocument])

  const simulateDocumentShare = useCallback(async (id: string, shareType: 'link' | 'email'): Promise<string> => {
    recordActivity('share', id)
    await new Promise(resolve => setTimeout(resolve, 200))
    
    const shareId = `share_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
    
    if (shareType === 'link') {
      return `https://app.readur.com/shared/${shareId}`
    } else {
      return `Share invitation sent via email (${shareId})`
    }
  }, [])

  // Testing utilities
  const generateTestDocuments = useCallback(async (scenario: DocumentTestScenario): Promise<MockDocument[]> => {
    const { count, types, withOcr, withErrors, recentUploads } = scenario
    
    if (count === 0) {
      clearAllDocuments()
      return []
    }

    const docs = createMockDocuments(count, {
      overrides: {
        ...(recentUploads && { 
          created_at: new Date(Date.now() - Math.random() * 24 * 60 * 60 * 1000).toISOString() 
        }),
      },
    })

    // Apply scenario modifications
    if (types && types.length > 0) {
      docs.forEach((doc, index) => {
        const type = types[index % types.length]
        doc.mime_type = getMimeTypeForType(type)
        doc.filename = `${doc.filename.split('.')[0]}.${getExtensionForType(type)}`
      })
    }

    if (withOcr) {
      docs.forEach(doc => {
        doc.ocr_status = Math.random() > 0.1 ? 'completed' : 'failed'
        if (doc.ocr_status === 'completed') {
          doc.ocr_text = `Mock OCR text for ${doc.filename}`
          doc.ocr_confidence = 0.8 + Math.random() * 0.2
        }
      })
    }

    if (withErrors) {
      docs.slice(0, Math.floor(count * 0.2)).forEach(doc => {
        doc.processing_status = 'error'
        doc.error_message = 'Mock processing error'
      })
    }

    setDocuments(docs)
    return docs
  }, [clearAllDocuments])

  const resetToScenario = useCallback((scenarioName: string) => {
    const scenario = DOCUMENT_TEST_SCENARIOS[scenarioName]
    if (scenario) {
      generateTestDocuments(scenario)
    }
  }, [generateTestDocuments])

  const exportDocuments = useCallback((format: 'json' | 'csv'): string => {
    if (format === 'json') {
      return JSON.stringify(documents, null, 2)
    } else {
      // Simple CSV export
      const headers = ['id', 'filename', 'mime_type', 'size', 'created_at', 'ocr_status']
      const csvRows = [
        headers.join(','),
        ...documents.map(doc => headers.map(h => doc[h as keyof MockDocument]).join(','))
      ]
      return csvRows.join('\n')
    }
  }, [documents])

  const importDocuments = useCallback(async (data: string, format: 'json' | 'csv'): Promise<MockDocument[]> => {
    setIsLoading(true)
    
    let importedDocs: MockDocument[]
    
    if (format === 'json') {
      importedDocs = JSON.parse(data)
    } else {
      // Simple CSV parsing (in production, use a proper CSV parser)
      const lines = data.split('\n')
      const headers = lines[0].split(',')
      importedDocs = lines.slice(1).map(line => {
        const values = line.split(',')
        const doc: any = {}
        headers.forEach((header, index) => {
          doc[header] = values[index]
        })
        return doc as MockDocument
      })
    }
    
    setDocuments(prev => [...importedDocs, ...prev])
    setIsLoading(false)
    
    return importedDocs
  }, [])

  // Performance testing
  const stressTestDocuments = useCallback(async (count: number): Promise<void> => {
    console.log(`Starting stress test with ${count} documents`)
    const startTime = performance.now()
    
    await addMultipleDocuments(count)
    
    const endTime = performance.now()
    console.log(`Stress test completed in ${endTime - startTime}ms`)
  }, [addMultipleDocuments])

  const measureOperationTime = useCallback(async <T>(operation: () => Promise<T>): Promise<{ result: T; duration: number }> => {
    const startTime = performance.now()
    const result = await operation()
    const duration = performance.now() - startTime
    
    return { result, duration }
  }, [])

  // Statistics
  const getDocumentStats = useCallback((): DocumentStatistics => {
    const byType = documents.reduce((acc, doc) => {
      const type = doc.mime_type.split('/')[0]
      acc[type] = (acc[type] || 0) + 1
      return acc
    }, {} as Record<string, number>)

    const byStatus = documents.reduce((acc, doc) => {
      acc[doc.processing_status] = (acc[doc.processing_status] || 0) + 1
      return acc
    }, {} as Record<string, number>)

    const byUser = documents.reduce((acc, doc) => {
      acc[doc.user_id] = (acc[doc.user_id] || 0) + 1
      return acc
    }, {} as Record<string, number>)

    const totalSize = documents.reduce((sum, doc) => sum + doc.size, 0)
    const averageSize = documents.length > 0 ? totalSize / documents.length : 0

    const ocrDocs = documents.filter(doc => doc.ocr_status)
    const ocrSuccessRate = ocrDocs.length > 0 
      ? ocrDocs.filter(doc => doc.ocr_status === 'completed').length / ocrDocs.length 
      : 0

    // Generate upload trend (last 7 days)
    const uploadTrend = Array.from({ length: 7 }, (_, i) => {
      const date = new Date(Date.now() - i * 24 * 60 * 60 * 1000)
      const count = documents.filter(doc => {
        const docDate = new Date(doc.created_at)
        return docDate.toDateString() === date.toDateString()
      }).length
      return { date: date.toISOString().split('T')[0], count }
    }).reverse()

    return {
      total: documents.length,
      byType,
      byStatus,
      byUser,
      averageSize,
      totalSize,
      ocrSuccessRate,
      uploadTrend,
    }
  }, [documents])

  const getUsageMetrics = useCallback((): UsageMetrics => {
    const popularDocuments = documents
      .map(doc => ({
        ...doc,
        totalViews: usageMetrics.viewsPerDocument[doc.id] || 0,
      }))
      .sort((a, b) => b.totalViews - a.totalViews)
      .slice(0, 10)

    return {
      ...usageMetrics,
      popularDocuments,
    }
  }, [documents, usageMetrics])

  // Helper function to record activities
  const recordActivity = useCallback((action: string, documentId: string) => {
    setUsageMetrics(prev => ({
      ...prev,
      recentActivity: [
        { action, documentId, timestamp: new Date() },
        ...prev.recentActivity.slice(0, 49), // Keep last 50 activities
      ],
    }))
  }, [])

  return {
    documents: paginatedDocuments,
    filteredDocuments,
    totalCount,
    isLoading,
    addDocument,
    removeDocument,
    updateDocument,
    getDocument,
    addMultipleDocuments,
    removeMultipleDocuments,
    bulkUpdateDocuments,
    clearAllDocuments,
    setFilter,
    clearFilter,
    setSort,
    setPagination,
    simulateUploadProcess,
    simulateOcrProcess,
    simulateFailedOcr,
    simulateDuplicateDetection,
    simulateDocumentView,
    simulateDocumentDownload,
    simulateDocumentShare,
    generateTestDocuments,
    resetToScenario,
    exportDocuments,
    importDocuments,
    stressTestDocuments,
    measureOperationTime,
    getDocumentStats,
    getUsageMetrics,
  }
}

// Helper functions
function applyDocumentFilter(doc: MockDocument, filter: DocumentFilter): boolean {
  if (filter.search && !doc.filename.toLowerCase().includes(filter.search.toLowerCase())) {
    return false
  }
  if (filter.type && !doc.mime_type.includes(filter.type)) {
    return false
  }
  if (filter.status && doc.processing_status !== filter.status) {
    return false
  }
  if (filter.dateRange) {
    const docDate = new Date(doc.created_at)
    if (docDate < filter.dateRange.start || docDate > filter.dateRange.end) {
      return false
    }
  }
  if (filter.minSize && doc.size < filter.minSize) {
    return false
  }
  if (filter.maxSize && doc.size > filter.maxSize) {
    return false
  }
  if (filter.hasOcr !== undefined) {
    const hasOcr = doc.ocr_status === 'completed' && !!doc.ocr_text
    if (filter.hasOcr !== hasOcr) {
      return false
    }
  }
  if (filter.user && doc.user_id !== filter.user) {
    return false
  }
  return true
}

function applySorting(a: MockDocument, b: MockDocument, sort: DocumentSort): number {
  const aValue = a[sort.field]
  const bValue = b[sort.field]
  
  if (aValue < bValue) return sort.direction === 'asc' ? -1 : 1
  if (aValue > bValue) return sort.direction === 'asc' ? 1 : -1
  return 0
}

function getMimeTypeForType(type: string): string {
  const types: Record<string, string> = {
    pdf: 'application/pdf',
    image: 'image/jpeg',
    text: 'text/plain',
    office: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  }
  return types[type] || 'application/octet-stream'
}

function getExtensionForType(type: string): string {
  const extensions: Record<string, string> = {
    pdf: 'pdf',
    image: 'jpg',
    text: 'txt',
    office: 'docx',
  }
  return extensions[type] || 'bin'
}