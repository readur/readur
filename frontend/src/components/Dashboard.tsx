import React, { useState, useEffect } from 'react'
import FileUpload from './FileUpload'
import DocumentList from './DocumentList'
import SearchBar from './SearchBar'
import OcrAnalytics from './OcrAnalytics'
import { Document, documentService } from '../services/api'

function Dashboard() {
  const [documents, setDocuments] = useState<Document[]>([])
  const [loading, setLoading] = useState(true)
  const [searchResults, setSearchResults] = useState<Document[] | null>(null)

  useEffect(() => {
    loadDocuments()
  }, [])

  const loadDocuments = async () => {
    try {
      const response = await documentService.list()
      setDocuments(response.data)
    } catch (error) {
      console.error('Failed to load documents:', error)
    } finally {
      setLoading(false)
    }
  }

  const handleUploadSuccess = (newDocument: Document) => {
    setDocuments(prev => [newDocument, ...prev])
  }

  const handleSearch = async (query: string) => {
    if (!query.trim()) {
      setSearchResults(null)
      return
    }

    try {
      const response = await documentService.search({ query })
      // EnhancedDocument is a slimmer projection than Document; list rows only use the shared fields.
      setSearchResults(response.data.documents as unknown as Document[])
    } catch (error) {
      console.error('Search failed:', error)
    }
  }

  const displayDocuments = searchResults || documents

  return (
    <div className="px-4 py-6">
      <div className="mb-8">
        <h1 className="text-2xl font-bold text-gray-900 mb-4">Document Management</h1>
        <FileUpload onUploadSuccess={handleUploadSuccess} />
      </div>

      <div className="mb-6">
        <SearchBar onSearch={handleSearch} />
      </div>

      {!searchResults && (
        <div className="mb-6">
          <OcrAnalytics documents={documents} />
        </div>
      )}

      {searchResults && (
        <div className="mb-4">
          <button
            onClick={() => setSearchResults(null)}
            className="text-blue-600 hover:text-blue-500 text-sm"
          >
            ← Back to all documents
          </button>
        </div>
      )}

      <DocumentList documents={displayDocuments} loading={loading} />
    </div>
  )
}

export default Dashboard