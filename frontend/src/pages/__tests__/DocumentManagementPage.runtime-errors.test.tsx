import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import DocumentManagementPage from '../DocumentManagementPage';
import userEvent from '@testing-library/user-event';

// Mock API with comprehensive responses
vi.mock('../../services/api', () => ({
  api: {
    get: vi.fn(),
    delete: vi.fn(),
  },
  documentService: {
    getFailedDocuments: vi.fn(),
    getFailedOcrDocuments: vi.fn(),
    getDuplicates: vi.fn(),
    retryOcr: vi.fn(),
    deleteLowConfidence: vi.fn(),
    deleteFailedOcr: vi.fn(),
    downloadFile: vi.fn(),
  },
  queueService: {
    requeueFailed: vi.fn(),
  },
}));

const theme = createTheme();

const DocumentManagementPageWrapper = ({ children }: { children: React.ReactNode }) => {
  return (
    <BrowserRouter>
      <ThemeProvider theme={theme}>
        {children}
      </ThemeProvider>
    </BrowserRouter>
  );
};

describe('DocumentManagementPage - Runtime Error Prevention', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('OCR Confidence Display - Null Safety', () => {
    test('should handle null ocr_confidence without crashing', async () => {
      const mockFailedDocument = {
        id: 'test-doc-1',
        filename: 'test.pdf',
        original_filename: 'test.pdf',
        file_size: 1024,
        mime_type: 'application/pdf',
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        tags: [],
        failure_reason: 'low_ocr_confidence',
        failure_category: 'OCR Error',
        retry_count: 0,
        can_retry: true,
        ocr_confidence: null, // This should not cause a crash
        ocr_word_count: 10,
        error_message: 'Low confidence OCR result',
      };

      // Mock the API service
      const { documentService } = await import('../../services/api');
      vi.mocked(documentService.getFailedDocuments).mockResolvedValueOnce({
        data: {
          documents: [mockFailedDocument],
          pagination: { total: 1, limit: 25, offset: 0, total_pages: 1 },
          statistics: { total_failed: 1, by_reason: {}, by_stage: {} },
        },
      });

      render(
        <DocumentManagementPageWrapper>
          <DocumentManagementPage />
        </DocumentManagementPageWrapper>
      );

      // Wait for data to load
      await waitFor(() => {
        expect(screen.getByText('test.pdf')).toBeInTheDocument();
      });

      // Expand the row to see details
      const expandButton = screen.getByLabelText(/expand/i) || screen.getAllByRole('button')[0];
      fireEvent.click(expandButton);

      // Should not show confidence chip since ocr_confidence is null
      await waitFor(() => {
        expect(screen.queryByText(/confidence/)).not.toBeInTheDocument();
      });

      // But should show word count if available
      if (mockFailedDocument.ocr_word_count) {
        expect(screen.getByText(/10 words found/)).toBeInTheDocument();
      }
    });

    test('should handle undefined ocr_confidence without crashing', async () => {
      const mockFailedDocument = {
        id: 'test-doc-2',
        filename: 'test2.pdf',
        original_filename: 'test2.pdf',
        file_size: 1024,
        mime_type: 'application/pdf',
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        tags: [],
        failure_reason: 'low_ocr_confidence',
        failure_category: 'OCR Error',
        retry_count: 0,
        can_retry: true,
        // ocr_confidence is undefined
        ocr_word_count: undefined,
        error_message: 'Low confidence OCR result',
      };

      const { documentService } = await import('../../services/api');
      vi.mocked(documentService.getFailedDocuments).mockResolvedValueOnce({
        data: {
          documents: [mockFailedDocument],
          pagination: { total: 1, limit: 25, offset: 0, total_pages: 1 },
          statistics: { total_failed: 1, by_reason: {}, by_stage: {} },
        },
      });

      render(
        <DocumentManagementPageWrapper>
          <DocumentManagementPage />
        </DocumentManagementPageWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('test2.pdf')).toBeInTheDocument();
      });

      // Should render without crashing
      expect(screen.getByText('Document Management')).toBeInTheDocument();
    });

    test('should properly display valid ocr_confidence values', async () => {
      const mockFailedDocument = {
        id: 'test-doc-3',
        filename: 'test3.pdf',
        original_filename: 'test3.pdf',
        file_size: 1024,
        mime_type: 'application/pdf',
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        tags: [],
        failure_reason: 'low_ocr_confidence',
        failure_category: 'OCR Error',
        retry_count: 0,
        can_retry: true,
        ocr_confidence: 15.7, // Valid number
        ocr_word_count: 42,
        error_message: 'Low confidence OCR result',
      };

      const { documentService } = await import('../../services/api');
      vi.mocked(documentService.getFailedDocuments).mockResolvedValueOnce({
        data: {
          documents: [mockFailedDocument],
          pagination: { total: 1, limit: 25, offset: 0, total_pages: 1 },
          statistics: { total_failed: 1, by_reason: {}, by_stage: {} },
        },
      });

      render(
        <DocumentManagementPageWrapper>
          <DocumentManagementPage />
        </DocumentManagementPageWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('test3.pdf')).toBeInTheDocument();
      });

      // Expand the row to see details
      const expandButton = screen.getByLabelText(/expand/i) || screen.getAllByRole('button')[0];
      fireEvent.click(expandButton);

      // Should show confidence with proper formatting
      await waitFor(() => {
        expect(screen.getByText('15.7% confidence')).toBeInTheDocument();
        expect(screen.getByText('42 words found')).toBeInTheDocument();
      });
    });
  });

  describe('HTML Structure Validation', () => {
    test('should not nest block elements inside Typography components', async () => {
      const mockFailedDocument = {
        id: 'test-doc-4',
        filename: 'test4.pdf',
        original_filename: 'test4.pdf',
        file_size: 1024,
        mime_type: 'application/pdf',
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        tags: ['tag1', 'tag2'],
        failure_reason: 'low_ocr_confidence',
        failure_category: 'OCR Error',
        retry_count: 0,
        can_retry: true,
        ocr_confidence: 25.5,
        ocr_word_count: 15,
        error_message: 'Test error message',
      };

      const { documentService } = await import('../../services/api');
      vi.mocked(documentService.getFailedDocuments).mockResolvedValueOnce({
        data: {
          documents: [mockFailedDocument],
          pagination: { total: 1, limit: 25, offset: 0, total_pages: 1 },
          statistics: { total_failed: 1, by_reason: {}, by_stage: {} },
        },
      });

      render(
        <DocumentManagementPageWrapper>
          <DocumentManagementPage />
        </DocumentManagementPageWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('test4.pdf')).toBeInTheDocument();
      });

      // Click "View Details" to open the dialog
      const viewButton = screen.getByLabelText(/view details/i) || screen.getByText(/view details/i);
      fireEvent.click(viewButton);

      await waitFor(() => {
        expect(screen.getByText('Document Details: test4.pdf')).toBeInTheDocument();
      });

      // Check that tags are displayed correctly without HTML structure issues
      expect(screen.getByText('tag1')).toBeInTheDocument();
      expect(screen.getByText('tag2')).toBeInTheDocument();

      // Check that all sections render without throwing HTML validation errors
      expect(screen.getByText('Original Filename:')).toBeInTheDocument();
      expect(screen.getByText('File Size:')).toBeInTheDocument();
      expect(screen.getByText('MIME Type:')).toBeInTheDocument();
      expect(screen.getByText('Full Error Message:')).toBeInTheDocument();
    });
  });

  describe('Error Data Field Access', () => {
    test('should handle missing error_message field gracefully', async () => {
      const mockFailedDocument = {
        id: 'test-doc-5',
        filename: 'test5.pdf',
        original_filename: 'test5.pdf',
        file_size: 1024,
        mime_type: 'application/pdf',
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        tags: [],
        failure_reason: 'processing_error',
        failure_category: 'Processing Error',
        retry_count: 0,
        can_retry: true,
        // error_message is missing
        // ocr_error is missing too
      };

      const { documentService } = await import('../../services/api');
      vi.mocked(documentService.getFailedDocuments).mockResolvedValueOnce({
        data: {
          documents: [mockFailedDocument],
          pagination: { total: 1, limit: 25, offset: 0, total_pages: 1 },
          statistics: { total_failed: 1, by_reason: {}, by_stage: {} },
        },
      });

      render(
        <DocumentManagementPageWrapper>
          <DocumentManagementPage />
        </DocumentManagementPageWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('test5.pdf')).toBeInTheDocument();
      });

      // Expand the row to see details
      const expandButton = screen.getByLabelText(/expand/i) || screen.getAllByRole('button')[0];
      fireEvent.click(expandButton);

      // Should show fallback text
      await waitFor(() => {
        expect(screen.getByText('No error message available')).toBeInTheDocument();
      });
    });

    test('should prioritize error_message over ocr_error', async () => {
      const mockFailedDocument = {
        id: 'test-doc-6',
        filename: 'test6.pdf',
        original_filename: 'test6.pdf',
        file_size: 1024,
        mime_type: 'application/pdf',
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        tags: [],
        failure_reason: 'processing_error',
        failure_category: 'Processing Error',
        retry_count: 0,
        can_retry: true,
        error_message: 'New error message format',
        ocr_error: 'Old OCR error format',
      };

      const { documentService } = await import('../../services/api');
      vi.mocked(documentService.getFailedDocuments).mockResolvedValueOnce({
        data: {
          documents: [mockFailedDocument],
          pagination: { total: 1, limit: 25, offset: 0, total_pages: 1 },
          statistics: { total_failed: 1, by_reason: {}, by_stage: {} },
        },
      });

      render(
        <DocumentManagementPageWrapper>
          <DocumentManagementPage />
        </DocumentManagementPageWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('test6.pdf')).toBeInTheDocument();
      });

      // Expand the row to see details
      const expandButton = screen.getByLabelText(/expand/i) || screen.getAllByRole('button')[0];
      fireEvent.click(expandButton);

      // Should show the new error_message format, not ocr_error
      await waitFor(() => {
        expect(screen.getByText('New error message format')).toBeInTheDocument();
        expect(screen.queryByText('Old OCR error format')).not.toBeInTheDocument();
      });
    });

    test('should fallback to ocr_error when error_message is missing', async () => {
      const mockFailedDocument = {
        id: 'test-doc-7',
        filename: 'test7.pdf',
        original_filename: 'test7.pdf',
        file_size: 1024,
        mime_type: 'application/pdf',
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        tags: [],
        failure_reason: 'ocr_error',
        failure_category: 'OCR Error',
        retry_count: 0,
        can_retry: true,
        ocr_error: 'OCR processing failed',
        // error_message is missing
      };

      const { documentService } = await import('../../services/api');
      vi.mocked(documentService.getFailedDocuments).mockResolvedValueOnce({
        data: {
          documents: [mockFailedDocument],
          pagination: { total: 1, limit: 25, offset: 0, total_pages: 1 },
          statistics: { total_failed: 1, by_reason: {}, by_stage: {} },
        },
      });

      render(
        <DocumentManagementPageWrapper>
          <DocumentManagementPage />
        </DocumentManagementPageWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('test7.pdf')).toBeInTheDocument();
      });

      // Expand the row to see details
      const expandButton = screen.getByLabelText(/expand/i) || screen.getAllByRole('button')[0];
      fireEvent.click(expandButton);

      // Should show the OCR error
      await waitFor(() => {
        expect(screen.getByText('OCR processing failed')).toBeInTheDocument();
      });
    });
  });

  describe('Ignored Files Tab Functionality', () => {
    test('should render ignored files tab without errors', async () => {
      // Mock ignored files API responses
      const { api } = await import('../../services/api');
      vi.mocked(api.get).mockImplementation((url) => {
        if (url.includes('/ignored-files/stats')) {
          return Promise.resolve({
            data: {
              total_ignored_files: 5,
              total_size_bytes: 1024000,
              most_recent_ignored_at: '2024-01-01T00:00:00Z',
            }
          });
        }
        if (url.includes('/ignored-files')) {
          return Promise.resolve({
            data: {
              ignored_files: [],
              total: 0,
            }
          });
        }
        return Promise.resolve({ data: {} });
      });

      const { documentService } = await import('../../services/api');
      vi.mocked(documentService.getFailedDocuments).mockResolvedValueOnce({
        data: {
          documents: [],
          pagination: { total: 0, limit: 25, offset: 0, total_pages: 1 },
          statistics: { total_failed: 0, by_reason: {}, by_stage: {} },
        },
      });

      render(
        <DocumentManagementPageWrapper>
          <DocumentManagementPage />
        </DocumentManagementPageWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('Document Management')).toBeInTheDocument();
      });

      // Click on the Ignored Files tab
      const ignoredFilesTab = screen.getByText(/Ignored Files/);
      fireEvent.click(ignoredFilesTab);

      // Should render without errors
      await waitFor(() => {
        expect(screen.getByText('Ignored Files Management')).toBeInTheDocument();
      });
    });
  });

  describe('Edge Cases and Boundary Conditions', () => {
    test('should handle empty arrays and null values', async () => {
      const mockFailedDocument = {
        id: 'test-doc-8',
        filename: 'test8.pdf',
        original_filename: 'test8.pdf',
        file_size: 0, // Edge case: zero file size
        mime_type: 'application/pdf',
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        tags: [], // Empty array
        failure_reason: 'unknown',
        failure_category: 'Unknown',
        retry_count: 0,
        can_retry: false,
        ocr_confidence: 0, // Edge case: zero confidence
        ocr_word_count: 0, // Edge case: zero words
        error_message: '',
      };

      const { documentService } = await import('../../services/api');
      vi.mocked(documentService.getFailedDocuments).mockResolvedValueOnce({
        data: {
          documents: [mockFailedDocument],
          pagination: { total: 1, limit: 25, offset: 0, total_pages: 1 },
          statistics: { total_failed: 1, by_reason: {}, by_stage: {} },
        },
      });

      render(
        <DocumentManagementPageWrapper>
          <DocumentManagementPage />
        </DocumentManagementPageWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('test8.pdf')).toBeInTheDocument();
      });

      // Should render without crashing even with edge case values
      expect(screen.getByText('Document Management')).toBeInTheDocument();
    });

    test('should handle very large numbers without crashing', async () => {
      const mockFailedDocument = {
        id: 'test-doc-9',
        filename: 'test9.pdf',
        original_filename: 'test9.pdf',
        file_size: Number.MAX_SAFE_INTEGER, // Very large number
        mime_type: 'application/pdf',
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        tags: [],
        failure_reason: 'low_ocr_confidence',
        failure_category: 'OCR Error',
        retry_count: 999,
        can_retry: true,
        ocr_confidence: 99.999999, // High precision number
        ocr_word_count: 1000000, // Large word count
        error_message: 'Test error',
      };

      const { documentService } = await import('../../services/api');
      vi.mocked(documentService.getFailedDocuments).mockResolvedValueOnce({
        data: {
          documents: [mockFailedDocument],
          pagination: { total: 1, limit: 25, offset: 0, total_pages: 1 },
          statistics: { total_failed: 1, by_reason: {}, by_stage: {} },
        },
      });

      render(
        <DocumentManagementPageWrapper>
          <DocumentManagementPage />
        </DocumentManagementPageWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('test9.pdf')).toBeInTheDocument();
      });

      // Expand the row to see details
      const expandButton = screen.getByLabelText(/expand/i) || screen.getAllByRole('button')[0];
      fireEvent.click(expandButton);

      // Should handle large numbers gracefully
      await waitFor(() => {
        expect(screen.getByText('100.0% confidence')).toBeInTheDocument(); // Should be rounded properly
        expect(screen.getByText('1000000 words found')).toBeInTheDocument();
      });
    });
  });

  describe('Component Lifecycle and State Management', () => {
    test('should handle rapid tab switching without errors', async () => {
      const { documentService } = await import('../../services/api');
      const { api } = await import('../../services/api');
      
      // Mock all necessary API calls
      vi.mocked(documentService.getFailedDocuments).mockResolvedValue({
        data: {
          documents: [],
          pagination: { total: 0, limit: 25, offset: 0, total_pages: 1 },
          statistics: { total_failed: 0, by_reason: {}, by_stage: {} },
        },
      });

      vi.mocked(documentService.getDuplicates).mockResolvedValue({
        data: {
          duplicates: [],
          pagination: { total: 0, limit: 25, offset: 0, has_more: false },
          statistics: { total_duplicate_groups: 0 },
        },
      });

      vi.mocked(api.get).mockResolvedValue({
        data: {
          ignored_files: [],
          total: 0,
        }
      });

      const user = userEvent.setup();

      render(
        <DocumentManagementPageWrapper>
          <DocumentManagementPage />
        </DocumentManagementPageWrapper>
      );

      await waitFor(() => {
        expect(screen.getByText('Document Management')).toBeInTheDocument();
      });

      // Rapidly switch between tabs
      const tabs = screen.getAllByRole('tab');
      
      for (let i = 0; i < tabs.length; i++) {
        await user.click(tabs[i]);
        // Wait a minimal amount to ensure state updates
        await waitFor(() => {
          expect(tabs[i]).toHaveAttribute('aria-selected', 'true');
        });
      }

      // Should not crash or throw errors
      expect(screen.getByText('Document Management')).toBeInTheDocument();
    });
  });
});