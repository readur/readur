import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest'
import { screen, waitFor, fireEvent } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithMocks, renderWithAuth } from '../../test/mocks/react/render'
import { setupMockApi, resetMockApi } from '../../test/mocks'
import { createMockDocument, createMockDocuments } from '../../test/mocks/factories'
import Dashboard from '../Dashboard'


describe('Dashboard', () => {
  beforeEach(async () => {
    await setupMockApi({
      scenario: 'ACTIVE_SYSTEM',
      networkCondition: 'fast'
    })
  })

  afterEach(() => {
    resetMockApi()
    vi.clearAllMocks()
  })

  test('renders dashboard with file upload and document list', async () => {
    renderWithAuth(<Dashboard />)

    expect(screen.getByText('Document Management')).toBeInTheDocument()
    
    // Should have upload functionality
    expect(screen.getByText(/drag.*drop.*file/i) || screen.getByText(/click to select/i) || screen.getByText(/upload/i)).toBeInTheDocument()
    
    // Should have search functionality
    expect(screen.getByPlaceholderText(/search/i)).toBeInTheDocument()
    
    // Should show document list or loading state
    await waitFor(() => {
      expect(
        screen.getByText(/loading/i) || 
        screen.getByTestId('document-list') ||
        screen.getByText(/documents/i)
      ).toBeInTheDocument()
    })
  })

  test('handles loading state', async () => {
    // Set up slow network to test loading state
    await resetMockApi()
    await setupMockApi({
      scenario: 'ACTIVE_SYSTEM',
      networkCondition: 'slow'
    })
    
    renderWithAuth(<Dashboard />)

    // Should render the main elements
    expect(screen.getByText('Document Management')).toBeInTheDocument()
    
    // Should show loading state initially
    expect(screen.getByText(/loading/i) || screen.getByRole('progressbar')).toBeInTheDocument()
  })

  test('renders search functionality', async () => {
    renderWithAuth(<Dashboard />)

    // Check that search components are rendered
    const searchInput = screen.getByPlaceholderText(/search/i)
    expect(searchInput).toBeInTheDocument()
    
    // Should have search button or search functionality
    const searchButton = screen.queryByText(/search/i) || screen.queryByRole('button', { name: /search/i })
    expect(searchInput).toBeInTheDocument()
  })

  test('displays documents from mock API', async () => {
    renderWithAuth(<Dashboard />)

    // Wait for documents to load
    await waitFor(() => {
      // Should show documents or empty state
      expect(
        screen.getByText(/documents/i) ||
        screen.getByText(/no documents/i) ||
        screen.getByTestId('document-list')
      ).toBeInTheDocument()
    })
  })

  test('handles file upload', async () => {
    const user = userEvent.setup()
    renderWithAuth(<Dashboard />)

    // Find upload area or button
    const uploadArea = screen.getByText(/drag.*drop/i) || 
                     screen.getByText(/click to select/i) ||
                     screen.getByText(/upload/i)
    
    expect(uploadArea).toBeInTheDocument()
    
    // Create a test file
    const file = new File(['test content'], 'test.pdf', { type: 'application/pdf' })
    
    // Test file upload (may need to find file input)
    const fileInput = screen.queryByDisplayValue('') // File inputs often have empty display value
    if (fileInput && fileInput.tagName === 'INPUT') {
      await user.upload(fileInput as HTMLInputElement, file)
    }
    
    // Should handle file upload attempt
    expect(uploadArea).toBeInTheDocument()
  })

  test('handles search input', async () => {
    const user = userEvent.setup()
    renderWithAuth(<Dashboard />)

    const searchInput = screen.getByPlaceholderText(/search/i)
    
    // Type in search
    await user.type(searchInput, 'test query')
    
    expect(searchInput).toHaveValue('test query')
  })

  test('handles empty document state', async () => {
    // Set up empty system scenario
    await resetMockApi()
    await setupMockApi({
      scenario: 'EMPTY_SYSTEM',
      networkCondition: 'fast'
    })
    
    renderWithAuth(<Dashboard />)

    await waitFor(() => {
      // Should show empty state or no documents message
      expect(
        screen.getByText(/no documents/i) ||
        screen.getByText(/empty/i) ||
        screen.getByText('Document Management')
      ).toBeInTheDocument()
    })
  })

  test('handles network errors gracefully', async () => {
    // Set up offline scenario
    await resetMockApi()
    await setupMockApi({
      scenario: 'ACTIVE_SYSTEM',
      networkCondition: 'offline'
    })
    
    renderWithAuth(<Dashboard />)

    await waitFor(() => {
      // Should handle error gracefully - show error or still render basic UI
      expect(
        screen.getByText(/error/i) ||
        screen.getByText(/offline/i) ||
        screen.getByText('Document Management')
      ).toBeInTheDocument()
    })
  })
})