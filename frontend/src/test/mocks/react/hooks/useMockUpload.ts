/**
 * useMockUpload - File upload simulation for testing
 * Provides comprehensive file upload testing with realistic scenarios
 */

import { useState, useCallback, useRef } from 'react'
import { useMockApiContext } from '../providers/MockApiProvider'
import { useMockDocuments } from './useMockDocuments'
import { useMockWebSocketContext } from '../providers/MockWebSocketProvider'
import { createMockDocument } from '../../factories/document'
import type { MockDocument } from '../../api/types'

export interface UploadFile {
  id: string
  file: File
  status: 'pending' | 'uploading' | 'processing' | 'completed' | 'failed' | 'cancelled'
  progress: number
  speed: number // bytes per second
  estimatedTimeRemaining: number // seconds
  error?: string
  document?: MockDocument
}

export interface UploadOptions {
  chunkSize?: number
  maxConcurrentUploads?: number
  autoStartOcr?: boolean
  generateThumbnail?: boolean
  extractMetadata?: boolean
  validateFile?: boolean
  retryAttempts?: number
  enableCompression?: boolean
}

export interface UseMockUploadReturn {
  // Upload state
  uploads: UploadFile[]
  activeUploads: UploadFile[]
  completedUploads: UploadFile[]
  failedUploads: UploadFile[]
  totalProgress: number
  
  // Upload operations
  addFiles: (files: FileList | File[]) => string[]
  startUpload: (fileId: string) => Promise<void>
  startAllUploads: () => Promise<void>
  pauseUpload: (fileId: string) => void
  resumeUpload: (fileId: string) => void
  cancelUpload: (fileId: string) => void
  retryUpload: (fileId: string) => Promise<void>
  
  // Bulk operations
  clearCompleted: () => void
  clearFailed: () => void
  clearAll: () => void
  
  // Upload scenarios
  simulateSlowUpload: (fileId: string, speedKbps?: number) => Promise<void>
  simulateNetworkInterruption: (fileId: string, duration?: number) => Promise<void>
  simulateFileCorruption: (fileId: string) => Promise<void>
  simulateServerError: (fileId: string, errorType?: string) => Promise<void>
  simulateLargeFileUpload: (sizeGB: number) => Promise<string>
  
  // Drag and drop simulation
  simulateDragEnter: () => void
  simulateDragLeave: () => void
  simulateDrop: (files: File[]) => string[]
  
  // File validation
  validateFiles: (files: File[]) => FileValidationResult[]
  getAcceptedFileTypes: () => string[]
  getMaxFileSize: () => number
  
  // Upload analytics
  getUploadStats: () => UploadStatistics
  getSpeedHistory: (fileId: string) => number[]
  getThroughputMetrics: () => ThroughputMetrics
  
  // Configuration
  setUploadOptions: (options: Partial<UploadOptions>) => void
  resetUploads: () => void
  
  // Testing utilities
  stressTestUpload: (fileCount: number, fileSizeMB?: number) => Promise<void>
  benchmarkUpload: (files: File[]) => Promise<BenchmarkResult>
  simulateUploadQueue: (files: File[], concurrency?: number) => Promise<void>
}

export interface FileValidationResult {
  file: File
  isValid: boolean
  errors: string[]
  warnings: string[]
}

export interface UploadStatistics {
  totalFiles: number
  completedFiles: number
  failedFiles: number
  cancelledFiles: number
  totalBytes: number
  uploadedBytes: number
  averageSpeed: number
  successRate: number
  averageFileSize: number
}

export interface ThroughputMetrics {
  currentSpeed: number
  peakSpeed: number
  averageSpeed: number
  efficiency: number
  concurrentUploads: number
}

export interface BenchmarkResult {
  totalTime: number
  averageSpeed: number
  peakSpeed: number
  successRate: number
  files: Array<{
    name: string
    size: number
    uploadTime: number
    speed: number
    success: boolean
  }>
}

const DEFAULT_UPLOAD_OPTIONS: UploadOptions = {
  chunkSize: 1024 * 1024, // 1MB chunks
  maxConcurrentUploads: 3,
  autoStartOcr: true,
  generateThumbnail: true,
  extractMetadata: true,
  validateFile: true,
  retryAttempts: 3,
  enableCompression: false,
}

const ACCEPTED_FILE_TYPES = [
  'application/pdf',
  'image/jpeg',
  'image/png',
  'image/gif',
  'image/webp',
  'text/plain',
  'application/msword',
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  'application/vnd.ms-excel',
  'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
]

const MAX_FILE_SIZE = 100 * 1024 * 1024 // 100MB

export const useMockUpload = (): UseMockUploadReturn => {
  const [uploads, setUploads] = useState<UploadFile[]>([])
  const [uploadOptions, setUploadOptionsState] = useState<UploadOptions>(DEFAULT_UPLOAD_OPTIONS)
  const [dragActive, setDragActive] = useState(false)
  const [speedHistory, setSpeedHistory] = useState<Map<string, number[]>>(new Map())
  
  const apiContext = useMockApiContext?.()
  const { addDocument } = useMockDocuments()
  const webSocketContext = useMockWebSocketContext?.()
  const uploadIntervalsRef = useRef<Map<string, NodeJS.Timeout>>(new Map())

  // Computed state
  const activeUploads = uploads.filter(u => u.status === 'uploading' || u.status === 'processing')
  const completedUploads = uploads.filter(u => u.status === 'completed')
  const failedUploads = uploads.filter(u => u.status === 'failed')
  
  const totalProgress = uploads.length > 0 
    ? uploads.reduce((sum, upload) => sum + upload.progress, 0) / uploads.length 
    : 0

  // Upload operations
  const addFiles = useCallback((files: FileList | File[]): string[] => {
    const fileArray = Array.from(files)
    const newUploads: UploadFile[] = []
    
    fileArray.forEach(file => {
      const uploadFile: UploadFile = {
        id: `upload_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
        file,
        status: 'pending',
        progress: 0,
        speed: 0,
        estimatedTimeRemaining: 0,
      }
      newUploads.push(uploadFile)
    })
    
    setUploads(prev => [...prev, ...newUploads])
    return newUploads.map(u => u.id)
  }, [])

  const startUpload = useCallback(async (fileId: string): Promise<void> => {
    const upload = uploads.find(u => u.id === fileId)
    if (!upload || upload.status !== 'pending') return

    // Update status to uploading
    setUploads(prev => prev.map(u => 
      u.id === fileId ? { ...u, status: 'uploading' as const } : u
    ))

    try {
      // Validate file if enabled
      if (uploadOptions.validateFile) {
        const validation = validateSingleFile(upload.file)
        if (!validation.isValid) {
          throw new Error(validation.errors.join(', '))
        }
      }

      // Simulate chunked upload
      await simulateChunkedUpload(fileId, upload.file)
      
      // Create document after successful upload
      const document = await addDocument({
        filename: upload.file.name,
        mime_type: upload.file.type,
        size: upload.file.size,
        processing_status: 'processing',
        ocr_status: uploadOptions.autoStartOcr ? 'processing' : 'pending',
      })

      // Update upload with document reference
      setUploads(prev => prev.map(u => 
        u.id === fileId ? { 
          ...u, 
          status: 'processing' as const, 
          document,
          progress: 100 
        } : u
      ))

      // Simulate post-processing
      await simulatePostProcessing(fileId, document)

    } catch (error) {
      setUploads(prev => prev.map(u => 
        u.id === fileId ? { 
          ...u, 
          status: 'failed' as const, 
          error: error instanceof Error ? error.message : 'Upload failed' 
        } : u
      ))
    }
  }, [uploads, uploadOptions, addDocument])

  const simulateChunkedUpload = useCallback(async (fileId: string, file: File): Promise<void> => {
    const chunkSize = uploadOptions.chunkSize || DEFAULT_UPLOAD_OPTIONS.chunkSize!
    const totalChunks = Math.ceil(file.size / chunkSize)
    let uploadedChunks = 0
    
    const startTime = Date.now()
    const speeds: number[] = []

    return new Promise((resolve, reject) => {
      const uploadInterval = setInterval(() => {
        const upload = uploads.find(u => u.id === fileId)
        if (!upload || upload.status === 'cancelled') {
          clearInterval(uploadInterval)
          uploadIntervalsRef.current.delete(fileId)
          reject(new Error('Upload cancelled'))
          return
        }

        uploadedChunks++
        const progress = Math.min(100, (uploadedChunks / totalChunks) * 100)
        
        // Calculate speed (simulate variable network conditions)
        const elapsed = (Date.now() - startTime) / 1000
        const baseSpeed = 1024 * 1024 // 1MB/s base speed
        const networkVariation = 0.5 + Math.random() // 0.5x to 1.5x variation
        const currentSpeed = baseSpeed * networkVariation
        speeds.push(currentSpeed)
        
        // Update speed history
        setSpeedHistory(prev => {
          const newHistory = new Map(prev)
          const history = newHistory.get(fileId) || []
          history.push(currentSpeed)
          if (history.length > 20) history.shift() // Keep last 20 measurements
          newHistory.set(fileId, history)
          return newHistory
        })

        const remainingBytes = file.size * (1 - progress / 100)
        const estimatedTimeRemaining = remainingBytes / currentSpeed

        setUploads(prev => prev.map(u => 
          u.id === fileId ? { 
            ...u, 
            progress, 
            speed: currentSpeed,
            estimatedTimeRemaining 
          } : u
        ))

        // Send WebSocket progress update
        webSocketContext?.simulateUploadProgress?.(fileId, file.size)

        if (uploadedChunks >= totalChunks) {
          clearInterval(uploadInterval)
          uploadIntervalsRef.current.delete(fileId)
          resolve()
        }
      }, 100) // Update every 100ms

      uploadIntervalsRef.current.set(fileId, uploadInterval)
    })
  }, [uploads, uploadOptions, webSocketContext])

  const simulatePostProcessing = useCallback(async (fileId: string, document: MockDocument): Promise<void> => {
    // Simulate metadata extraction
    if (uploadOptions.extractMetadata) {
      await new Promise(resolve => setTimeout(resolve, 500))
    }

    // Simulate thumbnail generation
    if (uploadOptions.generateThumbnail && document.mime_type.startsWith('image/')) {
      await new Promise(resolve => setTimeout(resolve, 300))
    }

    // Simulate OCR processing
    if (uploadOptions.autoStartOcr) {
      await new Promise(resolve => setTimeout(resolve, 2000))
    }

    // Mark as completed
    setUploads(prev => prev.map(u => 
      u.id === fileId ? { ...u, status: 'completed' as const } : u
    ))
  }, [uploadOptions])

  const startAllUploads = useCallback(async (): Promise<void> => {
    const pendingUploads = uploads.filter(u => u.status === 'pending')
    const maxConcurrent = uploadOptions.maxConcurrentUploads || DEFAULT_UPLOAD_OPTIONS.maxConcurrentUploads!
    
    // Start uploads in batches
    for (let i = 0; i < pendingUploads.length; i += maxConcurrent) {
      const batch = pendingUploads.slice(i, i + maxConcurrent)
      await Promise.allSettled(batch.map(upload => startUpload(upload.id)))
    }
  }, [uploads, uploadOptions, startUpload])

  const pauseUpload = useCallback((fileId: string) => {
    const interval = uploadIntervalsRef.current.get(fileId)
    if (interval) {
      clearInterval(interval)
      uploadIntervalsRef.current.delete(fileId)
    }
    
    setUploads(prev => prev.map(u => 
      u.id === fileId && u.status === 'uploading' ? { ...u, status: 'pending' as const } : u
    ))
  }, [])

  const resumeUpload = useCallback((fileId: string) => {
    const upload = uploads.find(u => u.id === fileId)
    if (upload && upload.status === 'pending') {
      startUpload(fileId)
    }
  }, [uploads, startUpload])

  const cancelUpload = useCallback((fileId: string) => {
    const interval = uploadIntervalsRef.current.get(fileId)
    if (interval) {
      clearInterval(interval)
      uploadIntervalsRef.current.delete(fileId)
    }
    
    setUploads(prev => prev.map(u => 
      u.id === fileId ? { ...u, status: 'cancelled' as const } : u
    ))
  }, [])

  const retryUpload = useCallback(async (fileId: string): Promise<void> => {
    setUploads(prev => prev.map(u => 
      u.id === fileId ? { 
        ...u, 
        status: 'pending' as const, 
        progress: 0, 
        error: undefined 
      } : u
    ))
    
    await startUpload(fileId)
  }, [startUpload])

  // Bulk operations
  const clearCompleted = useCallback(() => {
    setUploads(prev => prev.filter(u => u.status !== 'completed'))
  }, [])

  const clearFailed = useCallback(() => {
    setUploads(prev => prev.filter(u => u.status !== 'failed'))
  }, [])

  const clearAll = useCallback(() => {
    // Cancel all active uploads
    uploadIntervalsRef.current.forEach(interval => clearInterval(interval))
    uploadIntervalsRef.current.clear()
    
    setUploads([])
    setSpeedHistory(new Map())
  }, [])

  // Upload scenarios
  const simulateSlowUpload = useCallback(async (fileId: string, speedKbps: number = 56): Promise<void> => {
    // Override speed for this upload
    const upload = uploads.find(u => u.id === fileId)
    if (!upload) return

    const slowChunkSize = Math.max(1024, speedKbps * 128) // Convert to bytes per 100ms
    const originalChunkSize = uploadOptions.chunkSize

    setUploadOptionsState(prev => ({ ...prev, chunkSize: slowChunkSize }))
    
    try {
      await startUpload(fileId)
    } finally {
      setUploadOptionsState(prev => ({ ...prev, chunkSize: originalChunkSize }))
    }
  }, [uploads, uploadOptions, startUpload])

  const simulateNetworkInterruption = useCallback(async (fileId: string, duration: number = 3000): Promise<void> => {
    pauseUpload(fileId)
    await new Promise(resolve => setTimeout(resolve, duration))
    resumeUpload(fileId)
  }, [pauseUpload, resumeUpload])

  const simulateFileCorruption = useCallback(async (fileId: string): Promise<void> => {
    await new Promise(resolve => setTimeout(resolve, 1000))
    
    setUploads(prev => prev.map(u => 
      u.id === fileId ? { 
        ...u, 
        status: 'failed' as const, 
        error: 'File appears to be corrupted' 
      } : u
    ))
  }, [])

  const simulateServerError = useCallback(async (fileId: string, errorType: string = 'server_error'): Promise<void> => {
    await new Promise(resolve => setTimeout(resolve, 500))
    
    const errorMessages = {
      server_error: 'Internal server error',
      storage_full: 'Server storage is full',
      timeout: 'Upload timeout',
      permission_denied: 'Permission denied',
    }
    
    setUploads(prev => prev.map(u => 
      u.id === fileId ? { 
        ...u, 
        status: 'failed' as const, 
        error: errorMessages[errorType as keyof typeof errorMessages] || 'Unknown error' 
      } : u
    ))
  }, [])

  const simulateLargeFileUpload = useCallback(async (sizeGB: number): Promise<string> => {
    const largeFile = new File([''], `large-file-${sizeGB}GB.pdf`, {
      type: 'application/pdf',
    })
    
    // Override file size for simulation
    Object.defineProperty(largeFile, 'size', {
      value: sizeGB * 1024 * 1024 * 1024,
      writable: false,
    })
    
    const [fileId] = addFiles([largeFile])
    await startUpload(fileId)
    return fileId
  }, [addFiles, startUpload])

  // Drag and drop simulation
  const simulateDragEnter = useCallback(() => {
    setDragActive(true)
  }, [])

  const simulateDragLeave = useCallback(() => {
    setDragActive(false)
  }, [])

  const simulateDrop = useCallback((files: File[]): string[] => {
    setDragActive(false)
    return addFiles(files)
  }, [addFiles])

  // File validation
  const validateFiles = useCallback((files: File[]): FileValidationResult[] => {
    return files.map(file => validateSingleFile(file))
  }, [])

  const getAcceptedFileTypes = useCallback((): string[] => {
    return ACCEPTED_FILE_TYPES
  }, [])

  const getMaxFileSize = useCallback((): number => {
    return MAX_FILE_SIZE
  }, [])

  // Upload analytics
  const getUploadStats = useCallback((): UploadStatistics => {
    const totalBytes = uploads.reduce((sum, u) => sum + u.file.size, 0)
    const uploadedBytes = uploads.reduce((sum, u) => sum + (u.file.size * u.progress / 100), 0)
    const completedFiles = completedUploads.length
    const failedFiles = failedUploads.length
    const cancelledFiles = uploads.filter(u => u.status === 'cancelled').length
    
    const speeds = Array.from(speedHistory.values()).flat()
    const averageSpeed = speeds.length > 0 ? speeds.reduce((sum, s) => sum + s, 0) / speeds.length : 0
    
    return {
      totalFiles: uploads.length,
      completedFiles,
      failedFiles,
      cancelledFiles,
      totalBytes,
      uploadedBytes,
      averageSpeed,
      successRate: uploads.length > 0 ? completedFiles / uploads.length : 0,
      averageFileSize: uploads.length > 0 ? totalBytes / uploads.length : 0,
    }
  }, [uploads, completedUploads, failedUploads, speedHistory])

  const getSpeedHistory = useCallback((fileId: string): number[] => {
    return speedHistory.get(fileId) || []
  }, [speedHistory])

  const getThroughputMetrics = useCallback((): ThroughputMetrics => {
    const allSpeeds = Array.from(speedHistory.values()).flat()
    const currentSpeeds = activeUploads.map(u => u.speed)
    
    return {
      currentSpeed: currentSpeeds.reduce((sum, s) => sum + s, 0),
      peakSpeed: allSpeeds.length > 0 ? Math.max(...allSpeeds) : 0,
      averageSpeed: allSpeeds.length > 0 ? allSpeeds.reduce((sum, s) => sum + s, 0) / allSpeeds.length : 0,
      efficiency: activeUploads.length > 0 ? currentSpeeds.reduce((sum, s) => sum + s, 0) / activeUploads.length : 0,
      concurrentUploads: activeUploads.length,
    }
  }, [speedHistory, activeUploads])

  const setUploadOptions = useCallback((options: Partial<UploadOptions>) => {
    setUploadOptionsState(prev => ({ ...prev, ...options }))
  }, [])

  const resetUploads = useCallback(() => {
    clearAll()
    setUploadOptionsState(DEFAULT_UPLOAD_OPTIONS)
    setDragActive(false)
  }, [clearAll])

  // Testing utilities
  const stressTestUpload = useCallback(async (fileCount: number, fileSizeMB: number = 1): Promise<void> => {
    const files = Array.from({ length: fileCount }, (_, i) => 
      new File(['test content'], `stress-test-${i}.txt`, { type: 'text/plain' })
    )
    
    // Override file sizes
    files.forEach(file => {
      Object.defineProperty(file, 'size', {
        value: fileSizeMB * 1024 * 1024,
        writable: false,
      })
    })
    
    addFiles(files)
    await startAllUploads()
  }, [addFiles, startAllUploads])

  const benchmarkUpload = useCallback(async (files: File[]): Promise<BenchmarkResult> => {
    const startTime = performance.now()
    const fileIds = addFiles(files)
    
    const results = await Promise.allSettled(
      fileIds.map(id => startUpload(id))
    )
    
    const totalTime = performance.now() - startTime
    const successCount = results.filter(r => r.status === 'fulfilled').length
    const speeds = Array.from(speedHistory.values()).flat()
    
    return {
      totalTime,
      averageSpeed: speeds.reduce((sum, s) => sum + s, 0) / speeds.length,
      peakSpeed: Math.max(...speeds),
      successRate: successCount / files.length,
      files: files.map((file, index) => ({
        name: file.name,
        size: file.size,
        uploadTime: totalTime / files.length, // Simplified
        speed: speeds[index] || 0,
        success: results[index].status === 'fulfilled',
      })),
    }
  }, [addFiles, startUpload, speedHistory])

  const simulateUploadQueue = useCallback(async (files: File[], concurrency: number = 3): Promise<void> => {
    const originalConcurrency = uploadOptions.maxConcurrentUploads
    setUploadOptions({ maxConcurrentUploads: concurrency })
    
    try {
      addFiles(files)
      await startAllUploads()
    } finally {
      setUploadOptions({ maxConcurrentUploads: originalConcurrency })
    }
  }, [uploadOptions, setUploadOptions, addFiles, startAllUploads])

  return {
    uploads,
    activeUploads,
    completedUploads,
    failedUploads,
    totalProgress,
    addFiles,
    startUpload,
    startAllUploads,
    pauseUpload,
    resumeUpload,
    cancelUpload,
    retryUpload,
    clearCompleted,
    clearFailed,
    clearAll,
    simulateSlowUpload,
    simulateNetworkInterruption,
    simulateFileCorruption,
    simulateServerError,
    simulateLargeFileUpload,
    simulateDragEnter,
    simulateDragLeave,
    simulateDrop,
    validateFiles,
    getAcceptedFileTypes,
    getMaxFileSize,
    getUploadStats,
    getSpeedHistory,
    getThroughputMetrics,
    setUploadOptions,
    resetUploads,
    stressTestUpload,
    benchmarkUpload,
    simulateUploadQueue,
  }
}

// Helper functions
function validateSingleFile(file: File): FileValidationResult {
  const errors: string[] = []
  const warnings: string[] = []
  
  // Check file type
  if (!ACCEPTED_FILE_TYPES.includes(file.type)) {
    errors.push(`File type ${file.type} is not supported`)
  }
  
  // Check file size
  if (file.size > MAX_FILE_SIZE) {
    errors.push(`File size ${formatFileSize(file.size)} exceeds maximum ${formatFileSize(MAX_FILE_SIZE)}`)
  }
  
  if (file.size < 1024) {
    warnings.push('File is very small, may not contain useful content')
  }
  
  // Check filename
  if (file.name.length > 255) {
    errors.push('Filename is too long')
  }
  
  if (!/^[a-zA-Z0-9._-]+$/.test(file.name)) {
    warnings.push('Filename contains special characters')
  }
  
  return {
    file,
    isValid: errors.length === 0,
    errors,
    warnings,
  }
}

function formatFileSize(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB']
  let size = bytes
  let unitIndex = 0
  
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024
    unitIndex++
  }
  
  return `${size.toFixed(1)} ${units[unitIndex]}`
}