import React, { useCallback, useState } from 'react'
import { useDropzone } from 'react-dropzone'
import { DocumentArrowUpIcon } from '@heroicons/react/24/outline'
import { Document, documentService } from '../services/api'
import { useNotifications } from '../contexts/NotificationContext'
import LanguageSelector from './LanguageSelector'

interface FileUploadProps {
  onUploadSuccess: (document: Document) => void
}

function FileUpload({ onUploadSuccess }: FileUploadProps) {
  const [uploading, setUploading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [selectedLanguages, setSelectedLanguages] = useState<string[]>(['eng'])
  const [primaryLanguage, setPrimaryLanguage] = useState<string>('eng')
  const { addBatchNotification } = useNotifications()

  const onDrop = useCallback(async (acceptedFiles: File[]) => {
    const file = acceptedFiles[0]
    if (!file) return

    setUploading(true)
    setError(null)

    try {
      const response = await documentService.upload(file, selectedLanguages.length > 0 ? selectedLanguages : undefined)
      onUploadSuccess(response.data)
      
      // Trigger success notification
      addBatchNotification('success', 'upload', [{ name: file.name, success: true }])
    } catch (err: any) {
      setError(err.response?.data?.message || 'Upload failed')
      
      // Trigger error notification
      addBatchNotification('error', 'upload', [{ name: file.name, success: false }])
    } finally {
      setUploading(false)
    }
  }, [onUploadSuccess, addBatchNotification, selectedLanguages])

  const handleLanguagesChange = (languages: string[], primary?: string) => {
    setSelectedLanguages(languages)
    if (primary) {
      setPrimaryLanguage(primary)
    } else if (languages.length > 0) {
      setPrimaryLanguage(languages[0])
    }
  }

  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop,
    multiple: false,
    accept: {
      'application/pdf': ['.pdf'],
      'text/plain': ['.txt'],
      'image/*': ['.png', '.jpg', '.jpeg', '.tiff', '.bmp'],
      'application/msword': ['.doc'],
      'application/vnd.openxmlformats-officedocument.wordprocessingml.document': ['.docx'],
    },
  })

  return (
    <div className="w-full space-y-4">
      {/* Language Selector */}
      <LanguageSelector
        selectedLanguages={selectedLanguages}
        primaryLanguage={primaryLanguage}
        onLanguagesChange={handleLanguagesChange}
        disabled={uploading}
      />
      
      {/* File Upload Area */}
      <div
        {...getRootProps()}
        className={`border-2 border-dashed rounded-lg p-6 text-center cursor-pointer transition-colors ${
          isDragActive
            ? 'border-blue-500 bg-blue-50'
            : 'border-gray-300 hover:border-gray-400'
        } ${uploading ? 'opacity-50 pointer-events-none' : ''}`}
      >
        <input {...getInputProps()} />
        <DocumentArrowUpIcon className="mx-auto h-12 w-12 text-gray-400" />
        <p className="mt-2 text-sm text-gray-600">
          {isDragActive
            ? 'Drop the file here...'
            : 'Drag & drop a file here, or click to select'}
        </p>
        <p className="text-xs text-gray-500 mt-1">
          Supported: PDF, TXT, DOC, DOCX, PNG, JPG, JPEG, TIFF, BMP
        </p>
        {uploading && (
          <p className="text-blue-600 mt-2">Uploading and processing...</p>
        )}
      </div>
      {error && (
        <div className="mt-2 text-red-600 text-sm">{error}</div>
      )}
    </div>
  )
}

export default FileUpload