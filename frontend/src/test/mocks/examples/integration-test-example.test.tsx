/**
 * Example integration test using the mock API framework
 * Demonstrates realistic workflows and scenarios
 */

import { describe, test, expect, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { 
  renderWithMockApi,
  setupAuthenticatedTest,
  withPerformanceMonitoring,
  useMockAuth,
  useMockDocuments,
  useMockWebSocket,
} from '../test-utils'
import { 
  createMockDocument,
  createMockDocumentWithScenario,
  WebSocketTestUtils,
  PERFORMANCE_BENCHMARKS,
} from '../mocks'

// Mock components for demonstration
const MockSearchPage = () => {
  const [query, setQuery] = React.useState('')
  const [results, setResults] = React.useState([])
  const [loading, setLoading] = React.useState(false)

  const handleSearch = async () => {
    setLoading(true)
    try {
      const response = await fetch(`/api/search/enhanced?query=${encodeURIComponent(query)}`)
      const data = await response.json()
      setResults(data.documents || [])
    } catch (error) {
      console.error('Search failed:', error)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div>
      <input
        data-testid="search-input"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search documents..."
      />
      <button 
        data-testid="search-button" 
        onClick={handleSearch}
        disabled={loading}
      >
        {loading ? 'Searching...' : 'Search'}
      </button>
      <div data-testid="search-results">
        {results.map(doc => (
          <div key={doc.id} data-testid="search-result">
            <h3>{doc.filename}</h3>
            {doc.snippets?.map((snippet, i) => (
              <p key={i} data-testid="search-snippet">{snippet.text}</p>
            ))}
          </div>
        ))}
      </div>
    </div>
  )
}

const MockUploadPage = () => {
  const [uploadStatus, setUploadStatus] = React.useState('idle')
  const [uploadedDocs, setUploadedDocs] = React.useState([])

  const handleFileUpload = async (file: File) => {
    setUploadStatus('uploading')
    
    const formData = new FormData()
    formData.append('file', file)
    
    try {
      const response = await fetch('/api/documents', {
        method: 'POST',
        body: formData,
      })
      
      if (response.ok) {
        const newDoc = await response.json()
        setUploadedDocs(prev => [...prev, newDoc])
        setUploadStatus('success')
      } else {
        setUploadStatus('error')
      }
    } catch (error) {
      setUploadStatus('error')
    }
  }

  return (
    <div>
      <input
        data-testid="file-input"
        type="file"
        onChange={(e) => {
          const file = e.target.files?.[0]
          if (file) handleFileUpload(file)
        }}
      />
      <div data-testid="upload-status">{uploadStatus}</div>
      <div data-testid="uploaded-documents">
        {uploadedDocs.map(doc => (
          <div key={doc.id} data-testid="uploaded-doc">
            {doc.filename}
          </div>
        ))}
      </div>
    </div>
  )
}

const MockSyncProgress = ({ sourceId }: { sourceId: string }) => {
  const [progress, setProgress] = React.useState(null)
  const { webSocket, connectionState, lastMessage } = useMockWebSocket(
    `ws://localhost:8000/api/sources/${sourceId}/sync/progress/ws`
  )

  React.useEffect(() => {
    if (lastMessage?.type === 'progress') {
      setProgress(lastMessage.data)
    }
  }, [lastMessage])

  const startSync = async () => {
    await fetch(`/api/sources/${sourceId}/sync`, { method: 'POST' })
    
    // Start WebSocket simulation
    if (webSocket) {
      webSocket.startSyncProgressSimulation(sourceId, 'in_progress')
    }
  }

  return (
    <div>
      <div data-testid="connection-state">{connectionState}</div>
      <button data-testid="start-sync" onClick={startSync}>
        Start Sync
      </button>
      {progress && (
        <div data-testid="sync-progress">
          <div>Phase: {progress.phase}</div>
          <div>Progress: {progress.files_progress_percent}%</div>
          <div>Files: {progress.files_processed}/{progress.files_found}</div>
          <div>Status: {progress.is_active ? 'Active' : 'Inactive'}</div>
        </div>
      )}
    </div>
  )
}

describe('Integration Tests - Complete Workflows', () => {
  beforeEach(() => {
    // Setup realistic integration test environment
    setupAuthenticatedTest('user')
  })

  test('complete document search workflow', withPerformanceMonitoring(async () => {
    // Setup: Create documents with searchable content
    const searchableDocs = [
      createMockDocumentWithScenario('pdf_with_high_confidence_ocr'),
      createMockDocument({ 
        filename: 'invoice-2024.pdf',
        tags: ['invoice', 'important'],
        has_ocr_text: true,
      }),
      createMockDocument({
        filename: 'report-q1.pdf', 
        tags: ['report', 'quarterly'],
        has_ocr_text: true,
      }),
    ]

    const TestWrapper = () => {
      const mockDocs = useMockDocuments()
      
      React.useEffect(() => {
        mockDocs.setDocuments(searchableDocs)
      }, [mockDocs])

      return <MockSearchPage />
    }

    renderWithMockApi(<TestWrapper />, {
      scenario: 'ACTIVE_SYSTEM'
    })

    const user = userEvent.setup()

    // Step 1: Enter search query
    const searchInput = screen.getByTestId('search-input')
    await user.type(searchInput, 'invoice')

    // Step 2: Execute search
    const searchButton = screen.getByTestId('search-button')
    await user.click(searchButton)

    // Step 3: Wait for results
    await waitFor(() => {
      expect(screen.getByTestId('search-results')).toBeInTheDocument()
    })

    // Step 4: Verify search results
    await waitFor(() => {
      const results = screen.getAllByTestId('search-result')
      expect(results.length).toBeGreaterThan(0)
      
      // Should find the invoice document
      const invoiceResult = results.find(result => 
        result.textContent?.includes('invoice-2024.pdf')
      )
      expect(invoiceResult).toBeTruthy()
    })

    // Step 5: Verify snippets are present
    const snippets = screen.getAllByTestId('search-snippet')
    expect(snippets.length).toBeGreaterThan(0)
  }))

  test('document upload and processing workflow', withPerformanceMonitoring(async () => {
    renderWithMockApi(<MockUploadPage />, {
      scenario: 'ACTIVE_SYSTEM'
    })

    const user = userEvent.setup()

    // Step 1: Create a test file
    const testFile = new File(['test content'], 'test-document.pdf', {
      type: 'application/pdf'
    })

    // Step 2: Upload file
    const fileInput = screen.getByTestId('file-input')
    await user.upload(fileInput, testFile)

    // Step 3: Verify upload starts
    await waitFor(() => {
      expect(screen.getByTestId('upload-status')).toHaveTextContent('uploading')
    })

    // Step 4: Wait for upload completion
    await waitFor(() => {
      expect(screen.getByTestId('upload-status')).toHaveTextContent('success')
    }, { timeout: 5000 })

    // Step 5: Verify document appears in uploaded list
    await waitFor(() => {
      const uploadedDocs = screen.getAllByTestId('uploaded-doc')
      expect(uploadedDocs).toHaveLength(1)
      expect(uploadedDocs[0]).toHaveTextContent('test-document.pdf')
    })
  }))

  test('real-time sync progress monitoring', async () => {
    const sourceId = 'test-source-123'

    renderWithMockApi(<MockSyncProgress sourceId={sourceId} />, {
      scenario: 'ACTIVE_SYSTEM'
    })

    const user = userEvent.setup()

    // Step 1: Wait for WebSocket connection
    await waitFor(() => {
      expect(screen.getByTestId('connection-state')).toHaveTextContent('connected')
    }, { timeout: 3000 })

    // Step 2: Start sync process
    const startSyncButton = screen.getByTestId('start-sync')
    await user.click(startSyncButton)

    // Step 3: Wait for initial progress
    await waitFor(() => {
      expect(screen.getByTestId('sync-progress')).toBeInTheDocument()
    }, { timeout: 2000 })

    // Step 4: Verify progress updates
    await waitFor(() => {
      const progressElement = screen.getByTestId('sync-progress')
      expect(progressElement).toHaveTextContent('Phase: processing')
      expect(progressElement).toHaveTextContent('Status: Active')
    }, { timeout: 3000 })

    // Step 5: Wait for completion
    await waitFor(() => {
      const progressElement = screen.getByTestId('sync-progress')
      expect(progressElement).toHaveTextContent('Progress: 100%')
      expect(progressElement).toHaveTextContent('Status: Inactive')
    }, { timeout: 10000 })
  })

  test('error handling and recovery workflow', async () => {
    const TestComponent = () => {
      const [attempt, setAttempt] = React.useState(0)
      const mockAuth = useMockAuth()

      const handleRetry = () => {
        setAttempt(prev => prev + 1)
        
        // Simulate auth recovery after first attempt
        if (attempt === 0) {
          mockAuth.login({
            id: 'recovered-user',
            username: 'recovereduser',
            email: 'recovered@example.com',
            role: 'user'
          })
        }
      }

      React.useEffect(() => {
        // Simulate auth error initially
        if (attempt === 0) {
          mockAuth.logout()
        }
      }, [attempt, mockAuth])

      return (
        <div>
          <div data-testid="auth-status">
            {mockAuth.isAuthenticated ? 'Authenticated' : 'Not Authenticated'}
          </div>
          <button data-testid="retry-button" onClick={handleRetry}>
            Retry ({attempt})
          </button>
          {mockAuth.isAuthenticated && <MockSearchPage />}
        </div>
      )
    }

    renderWithMockApi(<TestComponent />, {
      scenario: 'ACTIVE_SYSTEM'
    })

    const user = userEvent.setup()

    // Step 1: Verify initial error state
    expect(screen.getByTestId('auth-status')).toHaveTextContent('Not Authenticated')

    // Step 2: Attempt recovery
    await user.click(screen.getByTestId('retry-button'))

    // Step 3: Verify recovery
    await waitFor(() => {
      expect(screen.getByTestId('auth-status')).toHaveTextContent('Authenticated')
    })

    // Step 4: Verify app functionality restored
    expect(screen.getByTestId('search-input')).toBeInTheDocument()
  })

  test('multi-user workflow simulation', async () => {
    const UserSession = ({ userId, username }: { userId: string, username: string }) => {
      const mockAuth = useMockAuth()
      
      React.useEffect(() => {
        mockAuth.login({
          id: userId,
          username,
          email: `${username}@example.com`,
          role: 'user'
        })
      }, [userId, username, mockAuth])

      return (
        <div data-testid={`user-session-${userId}`}>
          <div data-testid={`current-user-${userId}`}>
            {mockAuth.currentUser?.username}
          </div>
          <MockSearchPage />
        </div>
      )
    }

    const MultiUserApp = () => (
      <div>
        <UserSession userId="user1" username="alice" />
        <UserSession userId="user2" username="bob" />
      </div>
    )

    renderWithMockApi(<MultiUserApp />, {
      scenario: 'MULTI_USER_SYSTEM'
    })

    // Verify both user sessions are active
    await waitFor(() => {
      expect(screen.getByTestId('current-user-user1')).toHaveTextContent('alice')
      expect(screen.getByTestId('current-user-user2')).toHaveTextContent('bob')
    })

    // Verify each user has independent search functionality
    const searchInputs = screen.getAllByTestId('search-input')
    expect(searchInputs).toHaveLength(2)
  })

  test('performance monitoring integration', withPerformanceMonitoring(async () => {
    const startTime = performance.now()

    renderWithMockApi(<MockSearchPage />, {
      scenario: 'SYSTEM_UNDER_LOAD', // Large dataset
      networkCondition: 'realistic'
    })

    const user = userEvent.setup()

    // Perform search with large dataset
    await user.type(screen.getByTestId('search-input'), 'document')
    await user.click(screen.getByTestId('search-button'))

    await waitFor(() => {
      expect(screen.getByTestId('search-results')).toBeInTheDocument()
    })

    const endTime = performance.now()
    const duration = endTime - startTime

    // Performance assertions
    expect(duration).toBeLessThan(PERFORMANCE_BENCHMARKS.SEARCH_RESPONSE.slow)
    
    if (duration > PERFORMANCE_BENCHMARKS.SEARCH_RESPONSE.acceptable) {
      console.warn(`Search took ${duration.toFixed(2)}ms - consider optimization`)
    }
  }))
})