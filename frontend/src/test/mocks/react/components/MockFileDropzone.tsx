/**
 * MockFileDropzone - File upload testing component
 * Provides comprehensive file upload testing with drag & drop simulation
 */

import React, { useState, useCallback, useRef, useEffect, DragEvent, ChangeEvent } from 'react'
import { useMockUpload } from '../hooks/useMockUpload'

export interface MockFileDropzoneProps {
  onFilesAdded?: (files: File[]) => void
  onUploadComplete?: (fileId: string, document: any) => void
  onUploadError?: (fileId: string, error: string) => void
  acceptedFileTypes?: string[]
  maxFileSize?: number
  maxFiles?: number
  multiple?: boolean
  disabled?: boolean
  autoUpload?: boolean
  showProgress?: boolean
  showPreview?: boolean
  className?: string
  style?: React.CSSProperties
  children?: React.ReactNode
}

export interface FilePreview {
  id: string
  file: File
  preview?: string
  isImage: boolean
}

const defaultAcceptedTypes = [
  'application/pdf',
  'image/jpeg',
  'image/png',
  'image/gif',
  'text/plain',
  'application/msword',
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
]

export const MockFileDropzone: React.FC<MockFileDropzoneProps> = ({
  onFilesAdded,
  onUploadComplete,
  onUploadError,
  acceptedFileTypes = defaultAcceptedTypes,
  maxFileSize = 100 * 1024 * 1024, // 100MB
  maxFiles = 10,
  multiple = true,
  disabled = false,
  autoUpload = false,
  showProgress = true,
  showPreview = true,
  className = '',
  style = {},
  children,
}) => {
  const [isDragActive, setIsDragActive] = useState(false)
  const [dragCounter, setDragCounter] = useState(0)
  const [filePreviews, setFilePreviews] = useState<FilePreview[]>([])
  const [errors, setErrors] = useState<string[]>([])
  
  const fileInputRef = useRef<HTMLInputElement>(null)
  const dropzoneRef = useRef<HTMLDivElement>(null)
  
  const {
    uploads,
    addFiles,
    startUpload,
    startAllUploads,
    removeUpload,
    validateFiles,
    getAcceptedFileTypes,
    getMaxFileSize,
  } = useMockUpload()

  // Monitor upload status changes
  useEffect(() => {
    uploads.forEach(upload => {
      if (upload.status === 'completed' && upload.document) {
        onUploadComplete?.(upload.id, upload.document)
      } else if (upload.status === 'failed' && upload.error) {
        onUploadError?.(upload.id, upload.error)
      }
    })
  }, [uploads, onUploadComplete, onUploadError])

  // Generate file previews
  useEffect(() => {
    const generatePreviews = async () => {
      const previews: FilePreview[] = []
      
      for (const upload of uploads.filter(u => u.status === 'pending' || u.status === 'uploading')) {
        const isImage = upload.file.type.startsWith('image/')
        let preview: string | undefined
        
        if (isImage && showPreview) {
          try {
            preview = await createImagePreview(upload.file)
          } catch (error) {
            console.warn('Failed to create preview:', error)
          }
        }
        
        previews.push({
          id: upload.id,
          file: upload.file,
          preview,
          isImage,
        })
      }
      
      setFilePreviews(previews)
    }
    
    generatePreviews()
  }, [uploads, showPreview])

  const createImagePreview = (file: File): Promise<string> => {
    return new Promise((resolve, reject) => {
      const reader = new FileReader()
      reader.onload = () => resolve(reader.result as string)
      reader.onerror = reject
      reader.readAsDataURL(file)
    })
  }

  const validateAndProcessFiles = useCallback((files: File[]) => {
    setErrors([])
    const validationResults = validateFiles(files)
    const validFiles: File[] = []
    const newErrors: string[] = []

    validationResults.forEach(result => {
      if (result.isValid) {
        validFiles.push(result.file)
      } else {
        newErrors.push(`${result.file.name}: ${result.errors.join(', ')}`)
      }
    })

    // Check file count limit
    const currentFileCount = uploads.filter(u => u.status !== 'completed').length
    const totalFiles = currentFileCount + validFiles.length
    
    if (totalFiles > maxFiles) {
      newErrors.push(`Cannot add more than ${maxFiles} files. Currently have ${currentFileCount} files.`)
      return
    }

    if (newErrors.length > 0) {
      setErrors(newErrors)
      return
    }

    if (validFiles.length > 0) {
      const fileIds = addFiles(validFiles)
      onFilesAdded?.(validFiles)
      
      if (autoUpload) {
        setTimeout(() => startAllUploads(), 100)
      }
    }
  }, [uploads, addFiles, validateFiles, maxFiles, autoUpload, onFilesAdded, startAllUploads])

  // Drag and drop handlers
  const handleDragEnter = useCallback((e: DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    e.stopPropagation()
    
    setDragCounter(prev => prev + 1)
    
    if (e.dataTransfer.items && e.dataTransfer.items.length > 0) {
      setIsDragActive(true)
    }
  }, [])

  const handleDragLeave = useCallback((e: DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    e.stopPropagation()
    
    setDragCounter(prev => {
      const newCounter = prev - 1
      if (newCounter === 0) {
        setIsDragActive(false)
      }
      return newCounter
    })
  }, [])

  const handleDragOver = useCallback((e: DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    e.stopPropagation()
  }, [])

  const handleDrop = useCallback((e: DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    e.stopPropagation()
    
    setIsDragActive(false)
    setDragCounter(0)
    
    if (disabled) return

    const droppedFiles = Array.from(e.dataTransfer.files)
    if (droppedFiles.length > 0) {
      validateAndProcessFiles(droppedFiles)
    }
  }, [disabled, validateAndProcessFiles])

  // File input handler
  const handleFileInputChange = useCallback((e: ChangeEvent<HTMLInputElement>) => {
    const selectedFiles = Array.from(e.target.files || [])
    if (selectedFiles.length > 0) {
      validateAndProcessFiles(selectedFiles)
    }
    
    // Reset file input
    if (fileInputRef.current) {
      fileInputRef.current.value = ''
    }
  }, [validateAndProcessFiles])

  const handleClick = useCallback(() => {
    if (!disabled && fileInputRef.current) {
      fileInputRef.current.click()
    }
  }, [disabled])

  const handleRemoveFile = useCallback((fileId: string) => {
    const upload = uploads.find(u => u.id === fileId)
    if (upload && upload.status === 'pending') {
      removeUpload?.(fileId)
    }
  }, [uploads, removeUpload])

  const formatFileSize = (bytes: number): string => {
    const units = ['B', 'KB', 'MB', 'GB']
    let size = bytes
    let unitIndex = 0
    
    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024
      unitIndex++
    }
    
    return `${size.toFixed(1)} ${units[unitIndex]}`
  }

  const getDropzoneStyles = (): React.CSSProperties => ({
    border: `2px dashed ${isDragActive ? '#3b82f6' : disabled ? '#d1d5db' : '#e5e7eb'}`,
    borderRadius: '16px',
    padding: '2rem',
    textAlign: 'center',
    backgroundColor: isDragActive ? '#eff6ff' : disabled ? '#f9fafb' : '#fafafa',
    cursor: disabled ? 'not-allowed' : 'pointer',
    transition: 'all 0.2s ease-in-out',
    minHeight: '200px',
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    position: 'relative',
    fontFamily: 'Inter, system-ui, sans-serif',
    ...style,
  })

  return (
    <div data-testid="mock-file-dropzone">
      {/* Hidden file input */}
      <input
        ref={fileInputRef}
        type="file"
        multiple={multiple}
        accept={acceptedFileTypes.join(',')}
        onChange={handleFileInputChange}
        style={{ display: 'none' }}
        data-testid="file-input"
      />

      {/* Main dropzone */}
      <div
        ref={dropzoneRef}
        className={className}
        style={getDropzoneStyles()}
        onDragEnter={handleDragEnter}
        onDragLeave={handleDragLeave}
        onDragOver={handleDragOver}
        onDrop={handleDrop}
        onClick={handleClick}
        data-testid="dropzone-area"
        role="button"
        tabIndex={disabled ? -1 : 0}
        aria-label="File upload dropzone"
      >
        {children || (
          <>
            {/* Upload icon */}
            <svg
              width="48"
              height="48"
              fill="none"
              stroke={isDragActive ? '#3b82f6' : disabled ? '#9ca3af' : '#6b7280'}
              viewBox="0 0 24 24"
              style={{ marginBottom: '1rem' }}
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1.5}
                d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
              />
            </svg>

            <h3 style={{
              margin: '0 0 0.5rem 0',
              fontSize: '1.125rem',
              fontWeight: '600',
              color: disabled ? '#9ca3af' : '#374151',
            }}>
              {isDragActive ? 'Drop files here' : 'Upload documents'}
            </h3>

            <p style={{
              margin: '0 0 1rem 0',
              fontSize: '0.875rem',
              color: disabled ? '#9ca3af' : '#6b7280',
            }}>
              {disabled 
                ? 'Upload is disabled'
                : `Drag & drop files here, or click to select`
              }
            </p>

            <div style={{
              fontSize: '0.75rem',
              color: '#9ca3af',
              lineHeight: '1.4',
            }}>
              <div>Supported formats: PDF, Images, Documents</div>
              <div>Max file size: {formatFileSize(maxFileSize)}</div>
              <div>Max files: {maxFiles}</div>
            </div>
          </>
        )}
      </div>

      {/* Error messages */}
      {errors.length > 0 && (
        <div
          style={{
            marginTop: '1rem',
            padding: '0.75rem',
            backgroundColor: '#fef2f2',
            border: '1px solid #fecaca',
            borderRadius: '8px',
            color: '#991b1b',
          }}
          data-testid="upload-errors"
        >
          <h4 style={{ margin: '0 0 0.5rem 0', fontSize: '0.875rem', fontWeight: '600' }}>
            Upload Errors:
          </h4>
          <ul style={{ margin: 0, paddingLeft: '1.5rem', fontSize: '0.75rem' }}>
            {errors.map((error, index) => (
              <li key={index}>{error}</li>
            ))}
          </ul>
        </div>
      )}

      {/* File previews */}
      {filePreviews.length > 0 && (
        <div style={{ marginTop: '1.5rem' }} data-testid="file-previews">
          <h4 style={{
            margin: '0 0 1rem 0',
            fontSize: '0.875rem',
            fontWeight: '600',
            color: '#374151',
          }}>
            Files to Upload ({filePreviews.length})
          </h4>
          
          <div style={{
            display: 'grid',
            gap: '0.75rem',
            gridTemplateColumns: showPreview ? 'repeat(auto-fill, minmax(120px, 1fr))' : '1fr',
          }}>
            {filePreviews.map(preview => {
              const upload = uploads.find(u => u.id === preview.id)
              return (
                <FilePreviewCard
                  key={preview.id}
                  preview={preview}
                  upload={upload!}
                  onRemove={handleRemoveFile}
                  showProgress={showProgress}
                  showPreview={showPreview}
                />
              )
            })}
          </div>
        </div>
      )}

      {/* Upload controls */}
      {uploads.some(u => u.status === 'pending') && !autoUpload && (
        <div style={{ marginTop: '1rem', textAlign: 'center' }}>
          <button
            onClick={startAllUploads}
            style={{
              backgroundColor: '#3b82f6',
              color: 'white',
              border: 'none',
              padding: '0.75rem 1.5rem',
              borderRadius: '8px',
              fontSize: '0.875rem',
              fontWeight: '500',
              cursor: 'pointer',
              transition: 'background-color 0.2s',
            }}
            onMouseOver={(e) => e.currentTarget.style.backgroundColor = '#2563eb'}
            onMouseOut={(e) => e.currentTarget.style.backgroundColor = '#3b82f6'}
            data-testid="upload-all-button"
          >
            Upload All Files
          </button>
        </div>
      )}
    </div>
  )
}

// File preview card component
const FilePreviewCard: React.FC<{
  preview: FilePreview
  upload: any
  onRemove: (fileId: string) => void
  showProgress: boolean
  showPreview: boolean
}> = ({ preview, upload, onRemove, showProgress, showPreview }) => {
  const getStatusColor = (status: string) => {
    switch (status) {
      case 'uploading': return '#3b82f6'
      case 'processing': return '#f59e0b'
      case 'completed': return '#10b981'
      case 'failed': return '#ef4444'
      default: return '#6b7280'
    }
  }

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'uploading':
        return (
          <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16l-4-4m0 0l4-4m-4 4h18" />
          </svg>
        )
      case 'processing':
        return (
          <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
        )
      case 'completed':
        return (
          <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
          </svg>
        )
      case 'failed':
        return (
          <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        )
      default:
        return (
          <svg width="16" height="16" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
          </svg>
        )
    }
  }

  const formatFileSize = (bytes: number): string => {
    const units = ['B', 'KB', 'MB', 'GB']
    let size = bytes
    let unitIndex = 0
    
    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024
      unitIndex++
    }
    
    return `${size.toFixed(1)} ${units[unitIndex]}`
  }

  return (
    <div
      style={{
        border: '1px solid #e5e7eb',
        borderRadius: '8px',
        padding: '0.75rem',
        backgroundColor: 'white',
        position: 'relative',
        fontSize: '0.75rem',
        minHeight: showPreview ? '140px' : 'auto',
      }}
      data-testid={`file-preview-${preview.id}`}
    >
      {/* Remove button */}
      {upload.status === 'pending' && (
        <button
          onClick={() => onRemove(preview.id)}
          style={{
            position: 'absolute',
            top: '0.25rem',
            right: '0.25rem',
            background: '#ef4444',
            color: 'white',
            border: 'none',
            borderRadius: '50%',
            width: '20px',
            height: '20px',
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            fontSize: '12px',
          }}
          data-testid={`remove-file-${preview.id}`}
        >
          ×
        </button>
      )}

      {/* File preview or icon */}
      {showPreview && preview.isImage && preview.preview ? (
        <img
          src={preview.preview}
          alt={preview.file.name}
          style={{
            width: '100%',
            height: '60px',
            objectFit: 'cover',
            borderRadius: '4px',
            marginBottom: '0.5rem',
          }}
        />
      ) : (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            height: showPreview ? '60px' : '40px',
            backgroundColor: '#f3f4f6',
            borderRadius: '4px',
            marginBottom: '0.5rem',
            color: getStatusColor(upload.status),
          }}
        >
          {getStatusIcon(upload.status)}
        </div>
      )}

      {/* File info */}
      <div style={{ textAlign: 'center' }}>
        <div
          style={{
            fontWeight: '500',
            marginBottom: '0.25rem',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
          title={preview.file.name}
        >
          {preview.file.name}
        </div>
        
        <div style={{ color: '#6b7280', marginBottom: '0.25rem' }}>
          {formatFileSize(preview.file.size)}
        </div>

        <div
          style={{
            color: getStatusColor(upload.status),
            fontWeight: '500',
            textTransform: 'capitalize',
          }}
        >
          {upload.status}
        </div>

        {/* Progress bar */}
        {showProgress && upload.status === 'uploading' && (
          <div
            style={{
              marginTop: '0.5rem',
              height: '4px',
              backgroundColor: '#e5e7eb',
              borderRadius: '2px',
              overflow: 'hidden',
            }}
          >
            <div
              style={{
                width: `${upload.progress}%`,
                height: '100%',
                backgroundColor: getStatusColor(upload.status),
                transition: 'width 0.3s ease',
              }}
            />
          </div>
        )}

        {/* Error message */}
        {upload.status === 'failed' && upload.error && (
          <div
            style={{
              marginTop: '0.5rem',
              color: '#ef4444',
              fontSize: '0.625rem',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
            title={upload.error}
          >
            {upload.error}
          </div>
        )}
      </div>
    </div>
  )
}