/**
 * Example unit test using the mock API framework
 * Demonstrates basic testing patterns and utilities
 */

import { describe, test, expect, vi } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { 
  renderWithMockApi, 
  renderWithEmptyState,
  renderWithActiveSystem,
  renderWithErrors,
  useMockApi,
  getDefaultTestUser,
} from '../test-utils'
import { createMockDocument, createMockDocumentWithScenario } from '../mocks'

// Mock a simple DocumentList component for demonstration
const MockDocumentList = () => {
  const [documents, setDocuments] = React.useState([])
  const [loading, setLoading] = React.useState(true)
  const [error, setError] = React.useState(null)

  React.useEffect(() => {
    fetch('/api/documents')
      .then(res => res.json())
      .then(data => {
        setDocuments(data.documents || [])
        setLoading(false)
      })
      .catch(err => {
        setError(err.message)
        setLoading(false)
      })
  }, [])

  if (loading) return <div data-testid="loading">Loading...</div>
  if (error) return <div data-testid="error">Error: {error}</div>
  
  return (
    <div data-testid="document-list">
      {documents.length === 0 ? (
        <div data-testid="empty-state">No documents found</div>
      ) : (
        documents.map(doc => (
          <div key={doc.id} data-testid="document-item">
            {doc.filename}
          </div>
        ))
      )}
    </div>
  )
}

describe('DocumentList Component (Unit Tests)', () => {
  test('renders loading state initially', () => {
    renderWithMockApi(<MockDocumentList />)
    expect(screen.getByTestId('loading')).toBeInTheDocument()
  })

  test('renders empty state when no documents', async () => {
    renderWithEmptyState(<MockDocumentList />)
    
    await waitFor(() => {
      expect(screen.getByTestId('empty-state')).toBeInTheDocument()
    })
  })

  test('renders document list with active system', async () => {
    renderWithActiveSystem(<MockDocumentList />)
    
    await waitFor(() => {
      expect(screen.getByTestId('document-list')).toBeInTheDocument()
      expect(screen.getAllByTestId('document-item')).toHaveLength(50) // Default mock count
    })
  })

  test('handles API errors gracefully', async () => {
    renderWithErrors(<MockDocumentList />)
    
    await waitFor(() => {
      expect(screen.getByTestId('error')).toBeInTheDocument()
    })
  })

  test('works with custom mock data', async () => {
    const customDocs = [
      createMockDocumentWithScenario('pdf_with_high_confidence_ocr'),
      createMockDocumentWithScenario('image_with_failed_ocr'),
    ]

    const TestWrapper = () => {
      const mockApi = useMockApi()
      
      React.useEffect(() => {
        // Set custom documents
        mockApi.setDocuments(customDocs)
      }, [mockApi])

      return <MockDocumentList />
    }

    renderWithMockApi(<TestWrapper />)
    
    await waitFor(() => {
      expect(screen.getAllByTestId('document-item')).toHaveLength(2)
    })
  })

  test('simulates slow network conditions', async () => {
    const TestWrapper = () => {
      const mockApi = useMockApi()
      
      React.useEffect(() => {
        mockApi.simulateSlowNetwork(1000) // 1 second delay
      }, [mockApi])

      return <MockDocumentList />
    }

    const startTime = performance.now()
    renderWithMockApi(<TestWrapper />)
    
    // Should show loading for at least 800ms due to network delay
    expect(screen.getByTestId('loading')).toBeInTheDocument()
    
    await waitFor(() => {
      expect(screen.getByTestId('document-list')).toBeInTheDocument()
    }, { timeout: 2000 })
    
    const endTime = performance.now()
    expect(endTime - startTime).toBeGreaterThan(800)
  })

  test('handles authentication scenarios', async () => {
    const user = getDefaultTestUser()
    
    renderWithMockApi(<MockDocumentList />, {
      scenario: 'ACTIVE_SYSTEM',
      authValues: {
        user,
        isAuthenticated: true,
        login: vi.fn(),
        logout: vi.fn(),
      }
    })
    
    await waitFor(() => {
      expect(screen.getByTestId('document-list')).toBeInTheDocument()
    })
  })

  test('supports different network conditions', async () => {
    // Test fast network
    const { rerender } = renderWithMockApi(<MockDocumentList />, {
      networkCondition: 'fast'
    })
    
    await waitFor(() => {
      expect(screen.getByTestId('document-list')).toBeInTheDocument()
    }, { timeout: 500 })

    // Test offline condition
    rerender(
      renderWithMockApi(<MockDocumentList />, {
        networkCondition: 'offline'
      }).container.firstChild
    )
    
    await waitFor(() => {
      expect(screen.getByTestId('error')).toBeInTheDocument()
    })
  })
})

describe('Advanced Mock API Usage', () => {
  test('custom scenario with specific documents', async () => {
    const pdfDocuments = Array.from({ length: 3 }, () => 
      createMockDocument({ mime_type: 'application/pdf' })
    )

    const TestComponent = () => {
      const mockApi = useMockApi({ scenario: 'EMPTY_SYSTEM' })
      
      React.useEffect(() => {
        mockApi.setDocuments(pdfDocuments)
      }, [mockApi])

      return <MockDocumentList />
    }

    renderWithMockApi(<TestComponent />)
    
    await waitFor(() => {
      expect(screen.getAllByTestId('document-item')).toHaveLength(3)
    })
  })

  test('error recovery scenarios', async () => {
    const TestComponent = () => {
      const [retryCount, setRetryCount] = React.useState(0)
      const mockApi = useMockApi()
      
      const handleRetry = () => {
        setRetryCount(prev => prev + 1)
        
        // Clear errors after first retry
        if (retryCount === 0) {
          mockApi.reset()
        }
      }

      React.useEffect(() => {
        // Simulate error on first load
        if (retryCount === 0) {
          mockApi.simulateNetworkError()
        }
      }, [retryCount, mockApi])

      return (
        <div>
          <MockDocumentList />
          <button onClick={handleRetry} data-testid="retry-button">
            Retry ({retryCount})
          </button>
        </div>
      )
    }

    renderWithMockApi(<TestComponent />)
    
    // Should show error initially
    await waitFor(() => {
      expect(screen.getByTestId('error')).toBeInTheDocument()
    })
    
    // Click retry
    const user = userEvent.setup()
    await user.click(screen.getByTestId('retry-button'))
    
    // Should recover and show documents
    await waitFor(() => {
      expect(screen.getByTestId('document-list')).toBeInTheDocument()
    })
  })

  test('progressive loading simulation', async () => {
    const stages = ['loading', 'processing', 'complete']
    let currentStage = 0

    const TestComponent = () => {
      const [stage, setStage] = React.useState(stages[0])
      const mockApi = useMockApi()
      
      React.useEffect(() => {
        const interval = setInterval(() => {
          currentStage = (currentStage + 1) % stages.length
          setStage(stages[currentStage])
          
          // Update mock delay based on stage
          const delays = { loading: 500, processing: 300, complete: 0 }
          mockApi.setNetworkConfig({ delay: delays[stages[currentStage]] })
        }, 1000)

        return () => clearInterval(interval)
      }, [mockApi])

      return (
        <div>
          <div data-testid="stage">{stage}</div>
          <MockDocumentList />
        </div>
      )
    }

    renderWithMockApi(<TestComponent />)
    
    // Test progresses through different stages
    expect(screen.getByTestId('stage')).toHaveTextContent('loading')
    
    await waitFor(() => {
      expect(screen.getByTestId('stage')).toHaveTextContent('processing')
    }, { timeout: 1500 })
  })
})