import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { BrowserRouter } from 'react-router-dom';
import FailedOcrPage from '../FailedOcrPage';

// Integration test focused specifically on download functionality
describe('FailedOcrPage Download Integration', () => {
  const mockDownloadFile = vi.fn();
  
  // Mock the API service with more realistic download behavior
  vi.mock('../../services/api', () => ({
    api: {
      get: vi.fn(),
    },
    documentService: {
      getFailedOcrDocuments: vi.fn(() => Promise.resolve({
        data: {
          documents: [
            {
              id: 'test-doc-1',
              filename: 'integration-test.pdf',
              original_filename: 'Integration Test.pdf',
              file_size: 2048000,
              mime_type: 'application/pdf',
              created_at: '2024-01-01T12:00:00Z',
              ocr_error: 'Integration test error message',
              ocr_failure_reason: 'test_failure',
              ocr_completed_at: '2024-01-01T12:05:00Z',
              tags: ['integration', 'test'],
            },
          ],
          pagination: { total: 1, limit: 25, offset: 0, has_more: false },
          statistics: { 
            total_failed: 1, 
            failure_categories: [{ reason: 'test_failure', count: 1 }] 
          },
        },
      })),
      getDuplicates: vi.fn(() => Promise.resolve({
        data: {
          duplicates: [],
          pagination: { total: 0, limit: 25, offset: 0, has_more: false },
          statistics: { total_duplicate_groups: 0 },
        },
      })),
      retryOcr: vi.fn(() => Promise.resolve({
        data: { success: true, message: 'OCR retry queued successfully' }
      })),
      downloadFile: mockDownloadFile,
    },
  }));

  const TestWrapper = ({ children }: { children: React.ReactNode }) => (
    <BrowserRouter>{children}</BrowserRouter>
  );

  beforeEach(() => {
    vi.clearAllMocks();
    mockDownloadFile.mockResolvedValue(undefined);
  });

  test('integration: complete download flow from failed OCR page', async () => {
    const user = userEvent.setup();
    
    render(
      <TestWrapper>
        <FailedOcrPage />
      </TestWrapper>
    );

    // 1. Page loads successfully
    await waitFor(() => {
      expect(screen.getByText('Failed OCR & Duplicates')).toBeInTheDocument();
    });

    // 2. Failed documents are displayed
    await waitFor(() => {
      expect(screen.getByText('Integration Test.pdf')).toBeInTheDocument();
    });

    // 3. Download button is present and clickable
    const downloadButton = await screen.findByLabelText(/download/i);
    expect(downloadButton).toBeInTheDocument();
    expect(downloadButton).toBeEnabled();

    // 4. Click download triggers the service call
    await user.click(downloadButton);

    // 5. Verify download service was called with correct parameters
    expect(mockDownloadFile).toHaveBeenCalledTimes(1);
    expect(mockDownloadFile).toHaveBeenCalledWith(
      'test-doc-1',
      'Integration Test.pdf'
    );

    // 6. Page remains functional after download
    expect(screen.getByText('Integration Test.pdf')).toBeInTheDocument();
    expect(downloadButton).toBeEnabled();
  });

  test('integration: download error handling and recovery', async () => {
    const user = userEvent.setup();
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    
    // Mock download to fail
    mockDownloadFile.mockRejectedValueOnce(new Error('Network error'));
    
    render(
      <TestWrapper>
        <FailedOcrPage />
      </TestWrapper>
    );

    await waitFor(() => {
      expect(screen.getByText('Integration Test.pdf')).toBeInTheDocument();
    });

    const downloadButton = await screen.findByLabelText(/download/i);
    
    // Attempt download that will fail
    await user.click(downloadButton);

    // Verify error was handled
    await waitFor(() => {
      expect(mockDownloadFile).toHaveBeenCalled();
      expect(consoleErrorSpy).toHaveBeenCalledWith('Download failed:', expect.any(Error));
    });

    // Reset mock for successful retry
    mockDownloadFile.mockResolvedValueOnce(undefined);
    
    // Retry download should work
    await user.click(downloadButton);
    
    await waitFor(() => {
      expect(mockDownloadFile).toHaveBeenCalledTimes(2);
    });

    // Page should still be functional
    expect(screen.getByText('Integration Test.pdf')).toBeInTheDocument();
    
    consoleErrorSpy.mockRestore();
  });

  test('integration: download with authentication simulation', async () => {
    const user = userEvent.setup();
    
    // Simulate authentication check in download
    mockDownloadFile.mockImplementation(async (id, filename) => {
      // Simulate auth validation
      if (!id || !filename) {
        throw new Error('Unauthorized');
      }
      return Promise.resolve();
    });
    
    render(
      <TestWrapper>
        <FailedOcrPage />
      </TestWrapper>
    );

    await waitFor(() => {
      expect(screen.getByText('Integration Test.pdf')).toBeInTheDocument();
    });

    const downloadButton = await screen.findByLabelText(/download/i);
    await user.click(downloadButton);

    // Verify download was called with proper parameters (simulating auth success)
    expect(mockDownloadFile).toHaveBeenCalledWith(
      'test-doc-1',
      'Integration Test.pdf'
    );
  });

  test('integration: multiple downloads in sequence', async () => {
    const user = userEvent.setup();
    
    // Mock multiple documents
    vi.mocked(require('../../services/api').documentService.getFailedOcrDocuments).mockResolvedValueOnce({
      data: {
        documents: [
          {
            id: 'doc-1',
            filename: 'first.pdf',
            original_filename: 'First Document.pdf',
            file_size: 1024,
            mime_type: 'application/pdf',
            created_at: '2024-01-01T12:00:00Z',
            ocr_error: 'Error 1',
            ocr_failure_reason: 'reason_1',
            ocr_completed_at: '2024-01-01T12:01:00Z',
            tags: [],
          },
          {
            id: 'doc-2',
            filename: 'second.png',
            original_filename: 'Second Document.png',
            file_size: 2048,
            mime_type: 'image/png',
            created_at: '2024-01-01T12:10:00Z',
            ocr_error: 'Error 2',
            ocr_failure_reason: 'reason_2',
            ocr_completed_at: '2024-01-01T12:11:00Z',
            tags: [],
          },
        ],
        pagination: { total: 2, limit: 25, offset: 0, has_more: false },
        statistics: { 
          total_failed: 2, 
          failure_categories: [
            { reason: 'reason_1', count: 1 },
            { reason: 'reason_2', count: 1 }
          ] 
        },
      },
    });
    
    render(
      <TestWrapper>
        <FailedOcrPage />
      </TestWrapper>
    );

    // Wait for both documents to load
    await waitFor(() => {
      expect(screen.getByText('First Document.pdf')).toBeInTheDocument();
      expect(screen.getByText('Second Document.png')).toBeInTheDocument();
    });

    // Get all download buttons
    const downloadButtons = screen.getAllByLabelText(/download/i);
    expect(downloadButtons).toHaveLength(2);

    // Download first document
    await user.click(downloadButtons[0]);
    
    await waitFor(() => {
      expect(mockDownloadFile).toHaveBeenCalledWith('doc-1', 'First Document.pdf');
    });

    // Download second document
    await user.click(downloadButtons[1]);
    
    await waitFor(() => {
      expect(mockDownloadFile).toHaveBeenCalledWith('doc-2', 'Second Document.png');
    });

    // Verify both downloads were triggered
    expect(mockDownloadFile).toHaveBeenCalledTimes(2);
  });

  test('integration: download button accessibility', async () => {
    render(
      <TestWrapper>
        <FailedOcrPage />
      </TestWrapper>
    );

    await waitFor(() => {
      expect(screen.getByText('Integration Test.pdf')).toBeInTheDocument();
    });

    const downloadButton = await screen.findByLabelText(/download/i);
    
    // Verify accessibility attributes
    expect(downloadButton).toHaveAttribute('aria-label');
    expect(downloadButton.getAttribute('aria-label')).toMatch(/download/i);
    
    // Verify it's keyboard accessible
    downloadButton.focus();
    expect(document.activeElement).toBe(downloadButton);
    
    // Verify it can be activated with keyboard
    downloadButton.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }));
    downloadButton.dispatchEvent(new KeyboardEvent('keyup', { key: 'Enter' }));
    
    // Download should have been triggered
    await waitFor(() => {
      expect(mockDownloadFile).toHaveBeenCalled();
    });
  });
});