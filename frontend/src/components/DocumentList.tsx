import React from 'react'
import {
  DocumentIcon,
  PhotoIcon,
  ArrowDownTrayIcon,
} from '@heroicons/react/24/outline'
import { Document, documentService } from '../services/api'

interface DocumentListProps {
  documents: Document[]
  loading: boolean
}

function DocumentList({ documents, loading }: DocumentListProps) {
  const handleDownload = async (document: Document) => {
    try {
      const response = await documentService.download(document.id)
      const blob = new Blob([response.data])
      const url = window.URL.createObjectURL(blob)
      const link = window.document.createElement('a')
      link.href = url
      link.download = document.original_filename
      link.click()
      window.URL.revokeObjectURL(url)
    } catch (error) {
      console.error('Download failed:', error)
    }
  }

  const getFileIcon = (mimeType: string) => {
    if (mimeType.startsWith('image/')) {
      return <PhotoIcon className="h-8 w-8 text-green-500" />
    }
    return <DocumentIcon className="h-8 w-8 text-blue-500" />
  }

  const formatFileSize = (bytes: number) => {
    if (bytes === 0) return '0 Bytes'
    const k = 1024
    const sizes = ['Bytes', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
  }

  const getOcrStatusBadge = (document: Document) => {
    if (!document.has_ocr_text) {
      return null
    }

    const confidence = document.ocr_confidence
    const status = document.ocr_status

    if (status === 'failed') {
      return (
        <span className="ml-2 inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-red-100 text-red-800">
          OCR Failed
        </span>
      )
    }

    if (status === 'processing') {
      return (
        <span className="ml-2 inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-yellow-100 text-yellow-800">
          Processing...
        </span>
      )
    }

    if (confidence !== undefined) {
      let badgeClass = 'bg-green-100 text-green-800'
      let label = 'OCR'
      
      if (confidence >= 80) {
        badgeClass = 'bg-green-100 text-green-800'
        label = `OCR ${confidence.toFixed(0)}%`
      } else if (confidence >= 60) {
        badgeClass = 'bg-yellow-100 text-yellow-800'
        label = `OCR ${confidence.toFixed(0)}%`
      } else {
        badgeClass = 'bg-orange-100 text-orange-800'
        label = `OCR ${confidence.toFixed(0)}%`
      }

      return (
        <span className={`ml-2 inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${badgeClass}`}>
          {label}
        </span>
      )
    }

    return (
      <span className="ml-2 inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800">
        OCR
      </span>
    )
  }

  const getOcrMetrics = (document: Document) => {
    if (!document.has_ocr_text || document.ocr_word_count == null) {
      return null
    }

    const metrics = []

    metrics.push(`${document.ocr_word_count} words`)

    if (document.ocr_processing_time_ms) {
      const seconds = (document.ocr_processing_time_ms / 1000).toFixed(1)
      metrics.push(`${seconds}s`)
    }

    return metrics.length > 0 ? ` • ${metrics.join(' • ')}` : null
  }

  if (loading) {
    return (
      <div className="text-center py-8">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500 mx-auto"></div>
        <p className="mt-2 text-gray-600">Loading documents...</p>
      </div>
    )
  }

  if (documents.length === 0) {
    return (
      <div className="text-center py-8">
        <DocumentIcon className="mx-auto h-12 w-12 text-gray-400" />
        <p className="mt-2 text-gray-600">No documents found</p>
      </div>
    )
  }

  return (
    <div className="bg-white shadow overflow-hidden sm:rounded-md">
      <ul className="divide-y divide-gray-200">
        {documents.map((document) => (
          <li key={document.id}>
            <div className="px-4 py-4 flex items-center gap-4">
              <div className="flex items-center min-w-0 flex-1">
                {getFileIcon(document.mime_type)}
                <div className="ml-4 min-w-0 flex-1">
                  <div className="text-sm font-medium text-gray-900 truncate">
                    {document.original_filename}
                  </div>
                  <div className="text-sm text-gray-500">
                    {formatFileSize(document.file_size)} • {document.mime_type}
                    {getOcrMetrics(document)}
                    {getOcrStatusBadge(document)}
                  </div>
                  <div className="text-xs text-gray-400">
                    {new Date(document.created_at).toLocaleDateString()}
                  </div>
                </div>
              </div>
              <div className="flex-shrink-0">
                <button
                  onClick={() => handleDownload(document)}
                  className="inline-flex items-center p-2 border border-transparent rounded-full shadow-sm text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
                >
                  <ArrowDownTrayIcon className="h-4 w-4" />
                </button>
              </div>
            </div>
          </li>
        ))}
      </ul>
    </div>
  )
}

export default DocumentList