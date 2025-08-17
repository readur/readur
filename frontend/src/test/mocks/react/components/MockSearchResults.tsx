/**
 * MockSearchResults - Search UI testing component
 * Provides comprehensive search results display with realistic interactions
 */

import React, { useState, useCallback, useMemo } from 'react'
import { useMockSearch } from '../hooks/useMockSearch'
import type { MockSearchResult, SearchQuery } from '../hooks/useMockSearch'

export interface MockSearchResultsProps {
  query?: string
  onResultClick?: (result: MockSearchResult) => void
  onResultDownload?: (result: MockSearchResult) => void
  onResultShare?: (result: MockSearchResult) => void
  showSnippets?: boolean
  showThumbnails?: boolean
  showMetadata?: boolean
  showActions?: boolean
  highlightQuery?: boolean
  maxResults?: number
  itemsPerPage?: number
  enablePagination?: boolean
  sortBy?: 'relevance' | 'date' | 'name' | 'size'
  sortDirection?: 'asc' | 'desc'
  layout?: 'list' | 'grid' | 'compact'
  className?: string
  style?: React.CSSProperties
}

export const MockSearchResults: React.FC<MockSearchResultsProps> = ({
  query = '',
  onResultClick,
  onResultDownload,
  onResultShare,
  showSnippets = true,
  showThumbnails = true,
  showMetadata = true,
  showActions = true,
  highlightQuery = true,
  maxResults = 50,
  itemsPerPage = 10,
  enablePagination = true,
  sortBy = 'relevance',
  sortDirection = 'desc',
  layout = 'list',
  className = '',
  style = {},
}) => {
  const [currentPage, setCurrentPage] = useState(1)
  const [selectedResults, setSelectedResults] = useState<Set<string>>(new Set())
  const [viewMode, setViewMode] = useState<'normal' | 'debug'>(
    process.env.NODE_ENV === 'development' ? 'debug' : 'normal'
  )

  const {
    results,
    totalResults,
    isSearching,
    enhancedSearch,
    currentQuery,
    search,
  } = useMockSearch()

  // Auto-search when query changes
  React.useEffect(() => {
    if (query.trim()) {
      enhancedSearch(query, {
        sort: { field: sortBy, direction: sortDirection },
        highlight: highlightQuery,
      })
    }
  }, [query, sortBy, sortDirection, highlightQuery, enhancedSearch])

  // Sort and paginate results
  const processedResults = useMemo(() => {
    let processed = [...results]

    // Apply sorting
    processed.sort((a, b) => {
      let aValue: any, bValue: any
      
      switch (sortBy) {
        case 'relevance':
          aValue = a.relevance_score
          bValue = b.relevance_score
          break
        case 'date':
          aValue = new Date(a.document.created_at).getTime()
          bValue = new Date(b.document.created_at).getTime()
          break
        case 'name':
          aValue = a.document.filename.toLowerCase()
          bValue = b.document.filename.toLowerCase()
          break
        case 'size':
          aValue = a.document.size
          bValue = b.document.size
          break
        default:
          return 0
      }

      if (aValue < bValue) return sortDirection === 'asc' ? -1 : 1
      if (aValue > bValue) return sortDirection === 'asc' ? 1 : -1
      return 0
    })

    // Apply pagination
    if (enablePagination) {
      const startIndex = (currentPage - 1) * itemsPerPage
      const endIndex = startIndex + itemsPerPage
      processed = processed.slice(startIndex, endIndex)
    } else {
      processed = processed.slice(0, maxResults)
    }

    return processed
  }, [results, sortBy, sortDirection, currentPage, itemsPerPage, enablePagination, maxResults])

  const totalPages = enablePagination ? Math.ceil(totalResults / itemsPerPage) : 1

  // Event handlers
  const handleResultClick = useCallback((result: MockSearchResult, event: React.MouseEvent) => {
    event.preventDefault()
    onResultClick?.(result)
  }, [onResultClick])

  const handleResultSelect = useCallback((resultId: string, isSelected: boolean) => {
    setSelectedResults(prev => {
      const newSet = new Set(prev)
      if (isSelected) {
        newSet.add(resultId)
      } else {
        newSet.delete(resultId)
      }
      return newSet
    })
  }, [])

  const handleSelectAll = useCallback(() => {
    const allIds = processedResults.map(r => r.document.id)
    setSelectedResults(new Set(allIds))
  }, [processedResults])

  const handleDeselectAll = useCallback(() => {
    setSelectedResults(new Set())
  }, [])

  const handlePageChange = useCallback((page: number) => {
    setCurrentPage(page)
    // Scroll to top of results
    document.querySelector('[data-testid="search-results"]')?.scrollIntoView({ 
      behavior: 'smooth', 
      block: 'start' 
    })
  }, [])

  const highlightText = useCallback((text: string, query: string): string => {
    if (!highlightQuery || !query.trim()) return text
    
    const queryTerms = query.toLowerCase().split(/\s+/)
    let highlightedText = text
    
    queryTerms.forEach(term => {
      const regex = new RegExp(`(${term})`, 'gi')
      highlightedText = highlightedText.replace(regex, '<mark>$1</mark>')
    })
    
    return highlightedText
  }, [highlightQuery])

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

  const formatDate = (dateString: string): string => {
    const date = new Date(dateString)
    return date.toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    })
  }

  const getFileTypeIcon = (mimeType: string): string => {
    if (mimeType.startsWith('image/')) return '🖼️'
    if (mimeType.includes('pdf')) return '📄'
    if (mimeType.includes('word')) return '📝'
    if (mimeType.includes('excel') || mimeType.includes('spreadsheet')) return '📊'
    if (mimeType.includes('presentation')) return '📽️'
    if (mimeType.startsWith('text/')) return '📄'
    return '📋'
  }

  // Loading state
  if (isSearching) {
    return (
      <div 
        style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          padding: '3rem',
          minHeight: '200px',
          backgroundColor: '#fafafa',
          borderRadius: '12px',
          ...style,
        }}
        className={className}
        data-testid="search-loading"
      >
        <div
          style={{
            width: '32px',
            height: '32px',
            border: '3px solid #e5e7eb',
            borderTop: '3px solid #3b82f6',
            borderRadius: '50%',
            animation: 'spin 1s linear infinite',
            marginBottom: '1rem',
          }}
        />
        <p style={{ 
          margin: 0, 
          color: '#6b7280', 
          fontSize: '0.875rem',
          fontFamily: 'Inter, system-ui, sans-serif' 
        }}>
          Searching for "{query}"...
        </p>
        
        <style>
          {`
            @keyframes spin {
              to { transform: rotate(360deg); }
            }
          `}
        </style>
      </div>
    )
  }

  // Empty state
  if (!isSearching && totalResults === 0 && query.trim()) {
    return (
      <div
        style={{
          textAlign: 'center',
          padding: '3rem 2rem',
          backgroundColor: '#fafafa',
          borderRadius: '12px',
          border: '1px solid #e5e7eb',
          ...style,
        }}
        className={className}
        data-testid="search-empty"
      >
        <div style={{ fontSize: '48px', marginBottom: '1rem' }}>🔍</div>
        <h3 style={{ 
          margin: '0 0 0.5rem 0', 
          fontSize: '1.125rem', 
          fontWeight: '600',
          color: '#374151',
          fontFamily: 'Inter, system-ui, sans-serif',
        }}>
          No results found
        </h3>
        <p style={{ 
          margin: '0 0 1rem 0', 
          color: '#6b7280', 
          fontSize: '0.875rem' 
        }}>
          No documents match your search for "{query}"
        </p>
        <div style={{ fontSize: '0.75rem', color: '#9ca3af' }}>
          <p>Try:</p>
          <ul style={{ 
            listStyle: 'none', 
            padding: 0, 
            margin: '0.5rem 0 0 0',
            display: 'inline-block',
            textAlign: 'left',
          }}>
            <li>• Checking your spelling</li>
            <li>• Using different keywords</li>
            <li>• Trying broader search terms</li>
          </ul>
        </div>
      </div>
    )
  }

  return (
    <div 
      style={{ 
        fontFamily: 'Inter, system-ui, sans-serif',
        ...style 
      }}
      className={className}
      data-testid="search-results"
    >
      {/* Results header */}
      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        marginBottom: '1.5rem',
        padding: '0.75rem',
        backgroundColor: '#f8f9fa',
        borderRadius: '8px',
        fontSize: '0.875rem',
      }}>
        <div style={{ color: '#374151' }}>
          <strong>{totalResults.toLocaleString()}</strong> result{totalResults !== 1 ? 's' : ''} 
          {query && <span> for "<strong>{query}</strong>"</span>}
          {currentQuery && (
            <span style={{ color: '#6b7280', marginLeft: '0.5rem' }}>
              ({(Date.now() - (currentQuery as any).startTime || 0).toFixed(0)}ms)
            </span>
          )}
        </div>
        
        <div style={{ display: 'flex', gap: '0.75rem', alignItems: 'center' }}>
          {/* View mode toggle (development only) */}
          {process.env.NODE_ENV === 'development' && (
            <button
              onClick={() => setViewMode(prev => prev === 'normal' ? 'debug' : 'normal')}
              style={{
                background: viewMode === 'debug' ? '#3b82f6' : '#e5e7eb',
                color: viewMode === 'debug' ? 'white' : '#374151',
                border: 'none',
                padding: '0.25rem 0.5rem',
                borderRadius: '4px',
                fontSize: '0.75rem',
                cursor: 'pointer',
              }}
            >
              {viewMode === 'debug' ? '🐛' : '👁️'}
            </button>
          )}
          
          {/* Selection controls */}
          {showActions && processedResults.length > 0 && (
            <div style={{ display: 'flex', gap: '0.5rem' }}>
              <button
                onClick={handleSelectAll}
                style={{
                  background: 'transparent',
                  color: '#3b82f6',
                  border: '1px solid #3b82f6',
                  padding: '0.25rem 0.5rem',
                  borderRadius: '4px',
                  fontSize: '0.75rem',
                  cursor: 'pointer',
                }}
              >
                Select All
              </button>
              {selectedResults.size > 0 && (
                <button
                  onClick={handleDeselectAll}
                  style={{
                    background: '#ef4444',
                    color: 'white',
                    border: 'none',
                    padding: '0.25rem 0.5rem',
                    borderRadius: '4px',
                    fontSize: '0.75rem',
                    cursor: 'pointer',
                  }}
                >
                  Clear ({selectedResults.size})
                </button>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Results list */}
      <div
        style={{
          display: 'grid',
          gap: layout === 'grid' ? '1rem' : '0.75rem',
          gridTemplateColumns: layout === 'grid' 
            ? 'repeat(auto-fill, minmax(300px, 1fr))' 
            : '1fr',
        }}
      >
        {processedResults.map((result) => (
          <SearchResultItem
            key={result.document.id}
            result={result}
            query={query}
            isSelected={selectedResults.has(result.document.id)}
            onSelect={handleResultSelect}
            onClick={handleResultClick}
            onDownload={onResultDownload}
            onShare={onResultShare}
            showSnippets={showSnippets}
            showThumbnails={showThumbnails}
            showMetadata={showMetadata}
            showActions={showActions}
            highlightQuery={highlightQuery}
            layout={layout}
            viewMode={viewMode}
            highlightText={highlightText}
            formatFileSize={formatFileSize}
            formatDate={formatDate}
            getFileTypeIcon={getFileTypeIcon}
          />
        ))}
      </div>

      {/* Pagination */}
      {enablePagination && totalPages > 1 && (
        <div style={{
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          gap: '0.5rem',
          marginTop: '2rem',
          padding: '1rem',
        }}>
          <button
            onClick={() => handlePageChange(currentPage - 1)}
            disabled={currentPage === 1}
            style={{
              background: currentPage === 1 ? '#f3f4f6' : '#3b82f6',
              color: currentPage === 1 ? '#9ca3af' : 'white',
              border: 'none',
              padding: '0.5rem 0.75rem',
              borderRadius: '6px',
              fontSize: '0.875rem',
              cursor: currentPage === 1 ? 'not-allowed' : 'pointer',
            }}
          >
            Previous
          </button>
          
          <span style={{ 
            color: '#374151', 
            fontSize: '0.875rem',
            padding: '0 1rem',
          }}>
            Page {currentPage} of {totalPages}
          </span>
          
          <button
            onClick={() => handlePageChange(currentPage + 1)}
            disabled={currentPage === totalPages}
            style={{
              background: currentPage === totalPages ? '#f3f4f6' : '#3b82f6',
              color: currentPage === totalPages ? '#9ca3af' : 'white',
              border: 'none',
              padding: '0.5rem 0.75rem',
              borderRadius: '6px',
              fontSize: '0.875rem',
              cursor: currentPage === totalPages ? 'not-allowed' : 'pointer',
            }}
          >
            Next
          </button>
        </div>
      )}
    </div>
  )
}

// Individual search result item component
const SearchResultItem: React.FC<{
  result: MockSearchResult
  query: string
  isSelected: boolean
  onSelect: (id: string, selected: boolean) => void
  onClick?: (result: MockSearchResult, event: React.MouseEvent) => void
  onDownload?: (result: MockSearchResult) => void
  onShare?: (result: MockSearchResult) => void
  showSnippets: boolean
  showThumbnails: boolean
  showMetadata: boolean
  showActions: boolean
  highlightQuery: boolean
  layout: string
  viewMode: string
  highlightText: (text: string, query: string) => string
  formatFileSize: (bytes: number) => string
  formatDate: (date: string) => string
  getFileTypeIcon: (mimeType: string) => string
}> = ({
  result,
  query,
  isSelected,
  onSelect,
  onClick,
  onDownload,
  onShare,
  showSnippets,
  showThumbnails,
  showMetadata,
  showActions,
  highlightQuery,
  layout,
  viewMode,
  highlightText,
  formatFileSize,
  formatDate,
  getFileTypeIcon,
}) => {
  const handleCheckboxChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    e.stopPropagation()
    onSelect(result.document.id, e.target.checked)
  }

  const relevancePercentage = Math.round(result.relevance_score * 100)
  const isHighRelevance = result.relevance_score > 0.7

  return (
    <div
      style={{
        border: '1px solid #e5e7eb',
        borderRadius: '12px',
        padding: '1rem',
        backgroundColor: isSelected ? '#eff6ff' : 'white',
        cursor: 'pointer',
        transition: 'all 0.2s ease',
        position: 'relative',
      }}
      onClick={(e) => onClick?.(result, e)}
      onMouseOver={(e) => {
        if (!isSelected) {
          e.currentTarget.style.backgroundColor = '#f9fafb'
          e.currentTarget.style.borderColor = '#d1d5db'
        }
      }}
      onMouseOut={(e) => {
        if (!isSelected) {
          e.currentTarget.style.backgroundColor = 'white'
          e.currentTarget.style.borderColor = '#e5e7eb'
        }
      }}
      data-testid={`search-result-${result.document.id}`}
    >
      {/* Selection checkbox */}
      {showActions && (
        <input
          type="checkbox"
          checked={isSelected}
          onChange={handleCheckboxChange}
          style={{
            position: 'absolute',
            top: '0.75rem',
            right: '0.75rem',
            cursor: 'pointer',
          }}
          data-testid={`select-result-${result.document.id}`}
        />
      )}

      {/* Relevance indicator */}
      {viewMode === 'debug' && (
        <div
          style={{
            position: 'absolute',
            top: '0.5rem',
            left: '0.5rem',
            background: isHighRelevance ? '#10b981' : '#f59e0b',
            color: 'white',
            padding: '0.125rem 0.375rem',
            borderRadius: '12px',
            fontSize: '0.625rem',
            fontWeight: '600',
          }}
        >
          {relevancePercentage}%
        </div>
      )}

      <div style={{ 
        display: 'flex', 
        gap: '1rem',
        marginTop: viewMode === 'debug' ? '1.5rem' : '0',
      }}>
        {/* Thumbnail */}
        {showThumbnails && (
          <div style={{
            flexShrink: 0,
            width: '64px',
            height: '64px',
            backgroundColor: '#f3f4f6',
            borderRadius: '8px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            fontSize: '24px',
          }}>
            {getFileTypeIcon(result.document.mime_type)}
          </div>
        )}

        {/* Content */}
        <div style={{ flex: 1, minWidth: 0 }}>
          {/* Title */}
          <h3 style={{
            margin: '0 0 0.5rem 0',
            fontSize: '1rem',
            fontWeight: '600',
            color: '#1f2937',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}>
            <span
              dangerouslySetInnerHTML={{
                __html: highlightText(result.document.filename, query)
              }}
            />
          </h3>

          {/* Snippet */}
          {showSnippets && result.snippet && (
            <p style={{
              margin: '0 0 0.75rem 0',
              fontSize: '0.875rem',
              color: '#4b5563',
              lineHeight: '1.5',
              overflow: 'hidden',
              display: '-webkit-box',
              WebkitLineClamp: 2,
              WebkitBoxOrient: 'vertical',
            }}>
              <span
                dangerouslySetInnerHTML={{
                  __html: highlightText(result.snippet, query)
                }}
              />
            </p>
          )}

          {/* Metadata */}
          {showMetadata && (
            <div style={{
              display: 'flex',
              gap: '1rem',
              fontSize: '0.75rem',
              color: '#6b7280',
              marginBottom: '0.75rem',
            }}>
              <span>{formatFileSize(result.document.size)}</span>
              <span>{formatDate(result.document.created_at)}</span>
              {result.document.ocr_status === 'completed' && (
                <span style={{ color: '#10b981' }}>✓ Text extracted</span>
              )}
            </div>
          )}

          {/* Debug info */}
          {viewMode === 'debug' && (
            <div style={{
              fontSize: '0.625rem',
              color: '#9ca3af',
              fontFamily: 'monospace',
              marginTop: '0.5rem',
              padding: '0.5rem',
              backgroundColor: '#f9fafb',
              borderRadius: '4px',
            }}>
              <div>ID: {result.document.id}</div>
              <div>Score: {result.relevance_score.toFixed(4)}</div>
              <div>Type: {result.document.mime_type}</div>
              {result.document.ocr_confidence && (
                <div>OCR: {Math.round(result.document.ocr_confidence * 100)}%</div>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Actions */}
      {showActions && (
        <div style={{
          display: 'flex',
          gap: '0.5rem',
          marginTop: '0.75rem',
          paddingTop: '0.75rem',
          borderTop: '1px solid #f3f4f6',
        }}>
          <button
            onClick={(e) => {
              e.stopPropagation()
              onDownload?.(result)
            }}
            style={{
              background: '#3b82f6',
              color: 'white',
              border: 'none',
              padding: '0.375rem 0.75rem',
              borderRadius: '6px',
              fontSize: '0.75rem',
              cursor: 'pointer',
            }}
          >
            Download
          </button>
          
          <button
            onClick={(e) => {
              e.stopPropagation()
              onShare?.(result)
            }}
            style={{
              background: 'transparent',
              color: '#6b7280',
              border: '1px solid #d1d5db',
              padding: '0.375rem 0.75rem',
              borderRadius: '6px',
              fontSize: '0.75rem',
              cursor: 'pointer',
            }}
          >
            Share
          </button>
        </div>
      )}
    </div>
  )
}