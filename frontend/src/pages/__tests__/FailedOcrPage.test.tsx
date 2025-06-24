import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { BrowserRouter } from 'react-router-dom';
import FailedOcrPage from '../FailedOcrPage';

// Mock the API functions
const mockGetFailedOcrDocuments = vi.fn();
const mockGetDuplicates = vi.fn();
const mockRetryOcr = vi.fn();

vi.mock('../../services/api', () => ({
  documentService: {
    getFailedOcrDocuments: mockGetFailedOcrDocuments,
    getDuplicates: mockGetDuplicates,
    retryOcr: mockRetryOcr,
  },
}));

const mockFailedOcrResponse = {
  data: {
    documents: [
      {
        id: 'doc1',
        filename: 'test_document.pdf',
        original_filename: 'test_document.pdf',
        file_size: 1024000,
        mime_type: 'application/pdf',
        created_at: '2023-01-01T12:00:00Z',
        updated_at: '2023-01-01T12:30:00Z',
        tags: ['test', 'document'],
        ocr_status: 'failed',
        ocr_error: 'PDF font encoding issue: Missing unicode mapping for character',
        ocr_failure_reason: 'pdf_font_encoding',
        ocr_completed_at: '2023-01-01T12:30:00Z',
        retry_count: 1,
        last_attempt_at: '2023-01-01T12:30:00Z',
        can_retry: true,
        failure_category: 'PDF Font Issues',
      },
      {
        id: 'doc2',
        filename: 'corrupted_file.pdf',
        original_filename: 'corrupted_file.pdf',
        file_size: 2048000,
        mime_type: 'application/pdf',
        created_at: '2023-01-02T12:00:00Z',
        updated_at: '2023-01-02T12:30:00Z',
        tags: [],
        ocr_status: 'failed',
        ocr_error: 'PDF corruption detected: Corrupted internal structure',
        ocr_failure_reason: 'pdf_corruption',
        ocr_completed_at: '2023-01-02T12:30:00Z',
        retry_count: 2,
        last_attempt_at: '2023-01-02T12:30:00Z',
        can_retry: false,
        failure_category: 'PDF Corruption',
      },
    ],
    pagination: {
      total: 2,
      limit: 25,
      offset: 0,
      has_more: false,
    },
    statistics: {
      total_failed: 2,
      failure_categories: [
        { reason: 'pdf_font_encoding', display_name: 'PDF Font Issues', count: 1 },
        { reason: 'pdf_corruption', display_name: 'PDF Corruption', count: 1 },
      ],
    },
  },
};

const mockDuplicatesResponse = {
  data: {
    duplicates: [
      {
        file_hash: 'abc123def456',
        duplicate_count: 2,
        first_uploaded: '2023-01-01T12:00:00Z',
        last_uploaded: '2023-01-02T12:00:00Z',
        documents: [
          {
            id: 'dup1',
            filename: 'document_v1.pdf',
            original_filename: 'document_v1.pdf',
            file_size: 1024000,
            mime_type: 'application/pdf',
            created_at: '2023-01-01T12:00:00Z',
            user_id: 'user1',
          },
          {
            id: 'dup2',
            filename: 'document_v2.pdf',
            original_filename: 'document_v2.pdf',
            file_size: 1024000,
            mime_type: 'application/pdf',
            created_at: '2023-01-02T12:00:00Z',
            user_id: 'user1',
          },
        ],
      },
    ],
    pagination: {
      total: 1,
      limit: 25,
      offset: 0,
      has_more: false,
    },
    statistics: {
      total_duplicate_groups: 1,
    },
  },
};

const mockRetryResponse = {
  data: {
    success: true,
    message: 'OCR retry queued successfully',
    queue_id: 'queue123',
    estimated_wait_minutes: 5,
  },
};

const FailedOcrPageWrapper = ({ children }: { children: React.ReactNode }) => {
  return <BrowserRouter>{children}</BrowserRouter>;
};

describe('FailedOcrPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetFailedOcrDocuments.mockResolvedValue(mockFailedOcrResponse);
    mockGetDuplicates.mockResolvedValue(mockDuplicatesResponse);
    mockRetryOcr.mockResolvedValue(mockRetryResponse);
  });

  test('renders page title and tabs', async () => {
    render(
      <FailedOcrPageWrapper>
        <FailedOcrPage />
      </FailedOcrPageWrapper>
    );

    expect(screen.getByText('Failed OCR & Duplicates')).toBeInTheDocument();
    expect(screen.getByText(/Failed OCR/)).toBeInTheDocument();
    expect(screen.getByText(/Duplicates/)).toBeInTheDocument();
  });

  test('displays failed OCR statistics correctly', async () => {
    render(
      <FailedOcrPageWrapper>
        <FailedOcrPage />
      </FailedOcrPageWrapper>
    );

    await waitFor(() => {
      expect(screen.getByText('Total Failed')).toBeInTheDocument();
      expect(screen.getByText('2')).toBeInTheDocument();
      expect(screen.getByText('Failure Categories')).toBeInTheDocument();
      expect(screen.getByText('PDF Font Issues: 1')).toBeInTheDocument();
      expect(screen.getByText('PDF Corruption: 1')).toBeInTheDocument();
    });
  });

  test('displays failed documents in table', async () => {
    render(
      <FailedOcrPageWrapper>
        <FailedOcrPage />
      </FailedOcrPageWrapper>
    );

    await waitFor(() => {
      expect(screen.getByText('test_document.pdf')).toBeInTheDocument();
      expect(screen.getByText('corrupted_file.pdf')).toBeInTheDocument();
      expect(screen.getByText('1 attempts')).toBeInTheDocument();
      expect(screen.getByText('2 attempts')).toBeInTheDocument();
    });
  });

  test('shows success message when no failed documents', async () => {
    mockGetFailedOcrDocuments.mockResolvedValue({
      data: {
        documents: [],
        pagination: { total: 0, limit: 25, offset: 0, has_more: false },
        statistics: { total_failed: 0, failure_categories: [] },
      },
    });

    render(
      <FailedOcrPageWrapper>
        <FailedOcrPage />
      </FailedOcrPageWrapper>
    );

    await waitFor(() => {
      expect(screen.getByText('Great news!')).toBeInTheDocument();
      expect(screen.getByText(/No documents have failed OCR processing/)).toBeInTheDocument();
    });
  });

  test('handles retry OCR functionality', async () => {
    const user = userEvent.setup();
    
    render(
      <FailedOcrPageWrapper>
        <FailedOcrPage />
      </FailedOcrPageWrapper>
    );

    await waitFor(() => {
      expect(screen.getByText('test_document.pdf')).toBeInTheDocument();
    });

    // Click the retry button for the first document
    const retryButtons = screen.getAllByTitle('Retry OCR');
    await user.click(retryButtons[0]);

    expect(mockRetryOcr).toHaveBeenCalledWith('doc1');
    
    await waitFor(() => {
      expect(screen.getByText(/OCR retry queued for "test_document.pdf"/)).toBeInTheDocument();
    });
  });

  test('disables retry button when can_retry is false', async () => {
    render(
      <FailedOcrPageWrapper>
        <FailedOcrPage />
      </FailedOcrPageWrapper>
    );

    await waitFor(() => {
      const retryButtons = screen.getAllByTitle('Retry OCR');
      // The second document (corrupted_file.pdf) has can_retry: false
      expect(retryButtons[1]).toBeDisabled();
    });
  });

  test('switches to duplicates tab and displays duplicates', async () => {
    const user = userEvent.setup();
    
    render(
      <FailedOcrPageWrapper>
        <FailedOcrPage />
      </FailedOcrPageWrapper>
    );

    const duplicatesTab = screen.getByText(/Duplicates/);
    await user.click(duplicatesTab);

    await waitFor(() => {
      expect(screen.getByText('Total Duplicate Groups')).toBeInTheDocument();
      expect(screen.getByText('1')).toBeInTheDocument();
      expect(screen.getByText('document_v1.pdf')).toBeInTheDocument();
      expect(screen.getByText('document_v2.pdf')).toBeInTheDocument();
    });
  });

  test('expands and collapses document error details', async () => {
    const user = userEvent.setup();
    
    render(
      <FailedOcrPageWrapper>
        <FailedOcrPage />
      </FailedOcrPageWrapper>
    );

    await waitFor(() => {
      expect(screen.getByText('test_document.pdf')).toBeInTheDocument();
    });

    // Click the expand button for the first document
    const expandButtons = screen.getAllByRole('button', { name: '' });
    const expandButton = expandButtons.find(button => 
      button.querySelector('svg[data-testid="ExpandMoreIcon"]')
    );
    
    if (expandButton) {
      await user.click(expandButton);
      
      await waitFor(() => {
        expect(screen.getByText('Error Details')).toBeInTheDocument();
        expect(screen.getByText('PDF font encoding issue: Missing unicode mapping for character')).toBeInTheDocument();
      });
    }
  });

  test('handles API errors gracefully', async () => {
    mockGetFailedOcrDocuments.mockRejectedValue(new Error('API Error'));
    
    render(
      <FailedOcrPageWrapper>
        <FailedOcrPage />
      </FailedOcrPageWrapper>
    );

    await waitFor(() => {
      expect(screen.getByText('Failed to load failed OCR documents')).toBeInTheDocument();
    });
  });

  test('refreshes data when refresh button is clicked', async () => {
    const user = userEvent.setup();
    
    render(
      <FailedOcrPageWrapper>
        <FailedOcrPage />
      </FailedOcrPageWrapper>
    );

    await waitFor(() => {
      expect(mockGetFailedOcrDocuments).toHaveBeenCalledTimes(1);
    });

    const refreshButton = screen.getByText('Refresh');
    await user.click(refreshButton);

    expect(mockGetFailedOcrDocuments).toHaveBeenCalledTimes(2);
  });

  test('displays tab counts correctly', async () => {
    render(
      <FailedOcrPageWrapper>
        <FailedOcrPage />
      </FailedOcrPageWrapper>
    );

    await waitFor(() => {
      expect(screen.getByText('Failed OCR (2)')).toBeInTheDocument();
    });

    // Switch to duplicates tab to load duplicates data
    const user = userEvent.setup();
    const duplicatesTab = screen.getByText(/Duplicates/);
    await user.click(duplicatesTab);

    await waitFor(() => {
      expect(screen.getByText('Duplicates (1)')).toBeInTheDocument();
    });
  });

  test('displays appropriate failure category colors', async () => {
    render(
      <FailedOcrPageWrapper>
        <FailedOcrPage />
      </FailedOcrPageWrapper>
    );

    await waitFor(() => {
      const pdfFontChip = screen.getByText('PDF Font Issues');
      const pdfCorruptionChip = screen.getByText('PDF Corruption');
      
      expect(pdfFontChip).toBeInTheDocument();
      expect(pdfCorruptionChip).toBeInTheDocument();
    });
  });
});