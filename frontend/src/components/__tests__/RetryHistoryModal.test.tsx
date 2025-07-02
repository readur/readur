import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { RetryHistoryModal } from '../RetryHistoryModal';

// Mock the API
const mockGetDocumentRetryHistory = vi.fn();

const mockDocumentService = {
  getDocumentRetryHistory: mockGetDocumentRetryHistory,
};

vi.mock('../../services/api', () => ({
  documentService: mockDocumentService,
}));

describe('RetryHistoryModal', () => {
  const mockProps = {
    open: true,
    onClose: vi.fn(),
    documentId: 'test-doc-123',
    documentName: 'test-document.pdf',
  };

  const sampleRetryHistory = [
    {
      id: 'retry-1',
      retry_reason: 'bulk_retry_all',
      previous_status: 'failed',
      previous_failure_reason: 'low_confidence',
      previous_error: 'OCR confidence too low: 45%',
      priority: 15,
      queue_id: 'queue-1',
      created_at: '2024-01-15T10:30:00Z',
    },
    {
      id: 'retry-2',
      retry_reason: 'manual_retry',
      previous_status: 'failed',
      previous_failure_reason: 'image_quality',
      previous_error: 'Image resolution too low',
      priority: 12,
      queue_id: 'queue-2',
      created_at: '2024-01-14T14:20:00Z',
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
    mockGetDocumentRetryHistory.mockResolvedValue({
      data: {
        document_id: 'test-doc-123',
        retry_history: sampleRetryHistory,
        total_retries: 2,
      },
    });
  });

  test('renders modal with title and document name', () => {
    render(<RetryHistoryModal {...mockProps} />);

    expect(screen.getByText('OCR Retry History')).toBeInTheDocument();
    expect(screen.getByText('test-document.pdf')).toBeInTheDocument();
  });

  test('does not render when modal is closed', () => {
    render(<RetryHistoryModal {...mockProps} open={false} />);

    expect(screen.queryByText('OCR Retry History')).not.toBeInTheDocument();
  });

  test('loads and displays retry history on mount', async () => {
    render(<RetryHistoryModal {...mockProps} />);

    await waitFor(() => {
      expect(screen.getByText('Bulk Retry (All Documents)')).toBeInTheDocument();
    });

    expect(screen.getByText('Manual Retry')).toBeInTheDocument();
    expect(screen.getByText('Low Confidence')).toBeInTheDocument();
    expect(screen.getByText('Image Quality')).toBeInTheDocument();
    expect(screen.getByText('High')).toBeInTheDocument(); // Priority 15
    expect(screen.getByText('Medium')).toBeInTheDocument(); // Priority 12
  });

  test('shows loading state initially', () => {
    mockGetDocumentRetryHistory.mockImplementation(() => new Promise(() => {})); // Never resolves
    render(<RetryHistoryModal {...mockProps} />);

    expect(screen.getByRole('progressbar')).toBeInTheDocument();
    expect(screen.getByText('Loading retry history...')).toBeInTheDocument();
  });

  test('handles API errors gracefully', async () => {
    mockGetDocumentRetryHistory.mockRejectedValue(new Error('API Error'));
    render(<RetryHistoryModal {...mockProps} />);

    await waitFor(() => {
      expect(screen.getByText(/Failed to load retry history/)).toBeInTheDocument();
    });
  });

  test('shows empty state when no retry history exists', async () => {
    mockGetDocumentRetryHistory.mockResolvedValue({
      data: {
        document_id: 'test-doc-123',
        retry_history: [],
        total_retries: 0,
      },
    });

    render(<RetryHistoryModal {...mockProps} />);

    await waitFor(() => {
      expect(screen.getByText('No retry history found for this document.')).toBeInTheDocument();
    });
  });

  test('closes modal when close button is clicked', async () => {
    const user = userEvent.setup();
    render(<RetryHistoryModal {...mockProps} />);

    const closeButton = screen.getByText('Close');
    await user.click(closeButton);

    expect(mockProps.onClose).toHaveBeenCalled();
  });

  test('formats retry reasons correctly', async () => {
    const customHistory = [
      { ...sampleRetryHistory[0], retry_reason: 'bulk_retry_all' },
      { ...sampleRetryHistory[0], retry_reason: 'bulk_retry_specific' },
      { ...sampleRetryHistory[0], retry_reason: 'bulk_retry_filtered' },
      { ...sampleRetryHistory[0], retry_reason: 'manual_retry' },
      { ...sampleRetryHistory[0], retry_reason: 'unknown_reason' },
    ];

    mockGetDocumentRetryHistory.mockResolvedValue({
      data: {
        document_id: 'test-doc-123',
        retry_history: customHistory,
        total_retries: customHistory.length,
      },
    });

    render(<RetryHistoryModal {...mockProps} />);

    await waitFor(() => {
      expect(screen.getByText('Bulk Retry (All Documents)')).toBeInTheDocument();
      expect(screen.getByText('Bulk Retry (Specific Documents)')).toBeInTheDocument();
      expect(screen.getByText('Bulk Retry (Filtered)')).toBeInTheDocument();
      expect(screen.getByText('Manual Retry')).toBeInTheDocument();
      expect(screen.getByText('unknown_reason')).toBeInTheDocument(); // Unknown reasons show as-is
    });
  });

  test('formats priority levels correctly', async () => {
    const customHistory = [
      { ...sampleRetryHistory[0], priority: 20 },
      { ...sampleRetryHistory[0], priority: 15 },
      { ...sampleRetryHistory[0], priority: 10 },
      { ...sampleRetryHistory[0], priority: 5 },
      { ...sampleRetryHistory[0], priority: 1 },
    ];

    mockGetDocumentRetryHistory.mockResolvedValue({
      data: {
        document_id: 'test-doc-123',
        retry_history: customHistory,
        total_retries: customHistory.length,
      },
    });

    render(<RetryHistoryModal {...mockProps} />);

    await waitFor(() => {
      const highPriorities = screen.getAllByText('High');
      const mediumPriorities = screen.getAllByText('Medium');
      const lowPriorities = screen.getAllByText('Low');

      expect(highPriorities).toHaveLength(2); // Priority 20 and 15
      expect(mediumPriorities).toHaveLength(1); // Priority 10
      expect(lowPriorities).toHaveLength(2); // Priority 5 and 1
    });
  });

  test('formats failure reasons correctly', async () => {
    const customHistory = [
      { ...sampleRetryHistory[0], previous_failure_reason: 'low_confidence' },
      { ...sampleRetryHistory[0], previous_failure_reason: 'image_quality' },
      { ...sampleRetryHistory[0], previous_failure_reason: 'processing_timeout' },
      { ...sampleRetryHistory[0], previous_failure_reason: 'unknown_error' },
      { ...sampleRetryHistory[0], previous_failure_reason: null },
    ];

    mockGetDocumentRetryHistory.mockResolvedValue({
      data: {
        document_id: 'test-doc-123',
        retry_history: customHistory,
        total_retries: customHistory.length,
      },
    });

    render(<RetryHistoryModal {...mockProps} />);

    await waitFor(() => {
      expect(screen.getByText('Low Confidence')).toBeInTheDocument();
      expect(screen.getByText('Image Quality')).toBeInTheDocument();
      expect(screen.getByText('Processing Timeout')).toBeInTheDocument();
      expect(screen.getByText('Unknown Error')).toBeInTheDocument();
      expect(screen.getByText('N/A')).toBeInTheDocument(); // null reason
    });
  });

  test('displays previous error messages', async () => {
    render(<RetryHistoryModal {...mockProps} />);

    await waitFor(() => {
      expect(screen.getByText('OCR confidence too low: 45%')).toBeInTheDocument();
      expect(screen.getByText('Image resolution too low')).toBeInTheDocument();
    });
  });

  test('formats dates correctly', async () => {
    render(<RetryHistoryModal {...mockProps} />);

    await waitFor(() => {
      // Check that dates are formatted (exact format may vary by locale)
      expect(screen.getByText(/Jan/)).toBeInTheDocument();
      expect(screen.getByText(/2024/)).toBeInTheDocument();
    });
  });

  test('shows total retry count', async () => {
    render(<RetryHistoryModal {...mockProps} />);

    await waitFor(() => {
      expect(screen.getByText('Total retries: 2')).toBeInTheDocument();
    });
  });

  test('handles missing documentName gracefully', async () => {
    render(<RetryHistoryModal {...mockProps} documentName={undefined} />);

    await waitFor(() => {
      expect(screen.getByText('test-doc-123')).toBeInTheDocument(); // Falls back to documentId
    });
  });

  test('handles history entries with missing fields', async () => {
    const incompleteHistory = [
      {
        id: 'retry-1',
        retry_reason: null,
        previous_status: null,
        previous_failure_reason: null,
        previous_error: null,
        priority: null,
        queue_id: null,
        created_at: '2024-01-15T10:30:00Z',
      },
    ];

    mockGetDocumentRetryHistory.mockResolvedValue({
      data: {
        document_id: 'test-doc-123',
        retry_history: incompleteHistory,
        total_retries: 1,
      },
    });

    render(<RetryHistoryModal {...mockProps} />);

    await waitFor(() => {
      // Should not crash and should show N/A for missing fields
      expect(screen.getAllByText('N/A')).toHaveLength(4); // reason, failure reason, previous error, priority
    });
  });

  test('loads fresh data when documentId changes', async () => {
    const { rerender } = render(<RetryHistoryModal {...mockProps} />);

    await waitFor(() => {
      expect(mockGetDocumentRetryHistory).toHaveBeenCalledWith('test-doc-123');
    });

    // Change document ID
    rerender(<RetryHistoryModal {...mockProps} documentId="different-doc-456" />);

    await waitFor(() => {
      expect(mockGetDocumentRetryHistory).toHaveBeenCalledWith('different-doc-456');
    });

    expect(mockGetDocumentRetryHistory).toHaveBeenCalledTimes(2);
  });
});