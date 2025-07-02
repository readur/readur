import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Routes, Route } from 'react-router-dom';
import DocumentDetailsPage from '../DocumentDetailsPage';

// Mock the entire API module
const mockBulkRetryOcr = vi.fn();
const mockGetById = vi.fn();
const mockGetOcrText = vi.fn();
const mockGetThumbnail = vi.fn();
const mockGetDocumentRetryHistory = vi.fn();

const mockDocumentService = {
  getById: mockGetById,
  getOcrText: mockGetOcrText,
  getThumbnail: mockGetThumbnail,
  bulkRetryOcr: mockBulkRetryOcr,
  getDocumentRetryHistory: mockGetDocumentRetryHistory,
  download: vi.fn(),
  getProcessedImage: vi.fn(),
};

const mockApi = {
  get: vi.fn(),
  post: vi.fn(),
  put: vi.fn(),
};

vi.mock('../../services/api', () => ({
  documentService: mockDocumentService,
  default: mockApi,
}));

// Mock the RetryHistoryModal component
vi.mock('../../components/RetryHistoryModal', () => ({
  RetryHistoryModal: ({ open, onClose, documentId, documentName }: any) => (
    open ? (
      <div data-testid="retry-history-modal">
        <div>Retry History for {documentName}</div>
        <div>Document ID: {documentId}</div>
        <button onClick={onClose}>Close</button>
      </div>
    ) : null
  ),
}));

// Mock other components
vi.mock('../../components/DocumentViewer', () => ({
  default: ({ documentId, filename }: any) => (
    <div data-testid="document-viewer">
      Viewing {filename} (ID: {documentId})
    </div>
  ),
}));

vi.mock('../../components/Labels/LabelSelector', () => ({
  default: ({ selectedLabels, onLabelsChange }: any) => (
    <div data-testid="label-selector">
      <div>Selected: {selectedLabels.length} labels</div>
      <button onClick={() => onLabelsChange([])}>Clear Labels</button>
    </div>
  ),
}));

vi.mock('../../components/MetadataDisplay', () => ({
  default: ({ metadata, title }: any) => (
    <div data-testid="metadata-display">
      <h3>{title}</h3>
      <pre>{JSON.stringify(metadata, null, 2)}</pre>
    </div>
  ),
}));

describe('DocumentDetailsPage - Retry Functionality', () => {
  const mockDocument = {
    id: 'test-doc-1',
    original_filename: 'test-document.pdf',
    filename: 'test-document.pdf',
    file_size: 1024000,
    mime_type: 'application/pdf',
    created_at: '2023-01-01T00:00:00Z',
    has_ocr_text: true,
    tags: ['important'],
  };

  const mockOcrData = {
    document_id: 'test-doc-1',
    filename: 'test-document.pdf',
    has_ocr_text: true,
    ocr_text: 'Sample OCR text content',
    ocr_confidence: 95,
    ocr_word_count: 100,
    ocr_processing_time_ms: 5000,
    ocr_status: 'completed',
    ocr_completed_at: '2023-01-01T00:05:00Z',
  };

  beforeEach(() => {
    vi.clearAllMocks();
    
    mockGetById.mockResolvedValue({
      data: mockDocument,
    });

    mockGetOcrText.mockResolvedValue({
      data: mockOcrData,
    });

    mockGetThumbnail.mockRejectedValue(new Error('Thumbnail not available'));

    mockBulkRetryOcr.mockResolvedValue({
      data: {
        success: true,
        queued_count: 1,
        matched_count: 1,
        documents: [mockDocument],
        estimated_total_time_minutes: 2.0,
        message: 'OCR retry queued successfully',
      },
    });

    mockGetDocumentRetryHistory.mockResolvedValue({
      data: {
        document_id: 'test-doc-1',
        retry_history: [],
        total_retries: 0,
      },
    });

    mockApi.get.mockResolvedValue({ data: [] });
  });

  const renderDocumentDetailsPage = () => {
    return render(
      <MemoryRouter initialEntries={['/documents/test-doc-1']}>
        <Routes>
          <Route path="/documents/:id" element={<DocumentDetailsPage />} />
        </Routes>
      </MemoryRouter>
    );
  };

  test('renders retry OCR button', async () => {
    renderDocumentDetailsPage();

    await waitFor(() => {
      expect(screen.getByText('Document Details')).toBeInTheDocument();
    });

    expect(screen.getByText('Retry OCR')).toBeInTheDocument();
  });

  test('can retry OCR for document', async () => {
    const user = userEvent.setup();
    renderDocumentDetailsPage();

    await waitFor(() => {
      expect(screen.getByText('Document Details')).toBeInTheDocument();
    });

    const retryButton = screen.getByText('Retry OCR');
    expect(retryButton).toBeInTheDocument();
    
    // Clear previous calls to track only the retry call
    mockBulkRetryOcr.mockClear();

    await user.click(retryButton);

    await waitFor(() => {
      expect(mockBulkRetryOcr).toHaveBeenCalledWith({
        mode: 'specific',
        document_ids: ['test-doc-1'],
        priority_override: 15,
      });
    });
  });

  test('shows loading state during retry', async () => {
    const user = userEvent.setup();
    
    // Make the retry take some time
    mockBulkRetryOcr.mockImplementation(() => 
      new Promise(resolve => 
        setTimeout(() => resolve({
          data: {
            success: true,
            queued_count: 1,
            matched_count: 1,
            documents: [mockDocument],
            estimated_total_time_minutes: 2.0,
            message: 'OCR retry queued successfully',
          },
        }), 100)
      )
    );

    renderDocumentDetailsPage();

    await waitFor(() => {
      expect(screen.getByText('Document Details')).toBeInTheDocument();
    });

    const retryButton = screen.getByText('Retry OCR');
    await user.click(retryButton);

    // Should show loading state
    expect(screen.getByText('Retrying...')).toBeInTheDocument();
    
    // Wait for retry to complete
    await waitFor(() => {
      expect(screen.getByText('Retry OCR')).toBeInTheDocument();
    });
  });

  test('handles retry OCR error gracefully', async () => {
    const user = userEvent.setup();
    
    // Mock retry to fail
    mockBulkRetryOcr.mockRejectedValue(new Error('Retry failed'));

    renderDocumentDetailsPage();

    await waitFor(() => {
      expect(screen.getByText('Document Details')).toBeInTheDocument();
    });

    const retryButton = screen.getByText('Retry OCR');
    await user.click(retryButton);

    // Should still show the retry button (not stuck in loading state)
    await waitFor(() => {
      expect(screen.getByText('Retry OCR')).toBeInTheDocument();
    });

    expect(mockBulkRetryOcr).toHaveBeenCalled();
  });

  test('renders retry history button', async () => {
    renderDocumentDetailsPage();

    await waitFor(() => {
      expect(screen.getByText('Document Details')).toBeInTheDocument();
    });

    expect(screen.getByText('Retry History')).toBeInTheDocument();
  });

  test('can open retry history modal', async () => {
    const user = userEvent.setup();
    renderDocumentDetailsPage();

    await waitFor(() => {
      expect(screen.getByText('Document Details')).toBeInTheDocument();
    });

    const historyButton = screen.getByText('Retry History');
    await user.click(historyButton);

    // Should open the retry history modal
    expect(screen.getByTestId('retry-history-modal')).toBeInTheDocument();
    expect(screen.getByText('Retry History for test-document.pdf')).toBeInTheDocument();
    expect(screen.getByText('Document ID: test-doc-1')).toBeInTheDocument();
  });

  test('can close retry history modal', async () => {
    const user = userEvent.setup();
    renderDocumentDetailsPage();

    await waitFor(() => {
      expect(screen.getByText('Document Details')).toBeInTheDocument();
    });

    // Open modal
    const historyButton = screen.getByText('Retry History');
    await user.click(historyButton);

    expect(screen.getByTestId('retry-history-modal')).toBeInTheDocument();

    // Close modal
    const closeButton = screen.getByText('Close');
    await user.click(closeButton);

    expect(screen.queryByTestId('retry-history-modal')).not.toBeInTheDocument();
  });

  test('refreshes document details after successful retry', async () => {
    const user = userEvent.setup();
    
    // Mock successful retry
    mockBulkRetryOcr.mockResolvedValue({
      data: {
        success: true,
        queued_count: 1,
        matched_count: 1,
        documents: [mockDocument],
        estimated_total_time_minutes: 2.0,
        message: 'OCR retry queued successfully',
      },
    });

    renderDocumentDetailsPage();

    await waitFor(() => {
      expect(screen.getByText('Document Details')).toBeInTheDocument();
    });

    // Clear previous calls
    mockGetById.mockClear();

    const retryButton = screen.getByText('Retry OCR');
    await user.click(retryButton);

    // Should call getById again to refresh document details after delay
    await waitFor(() => {
      expect(mockGetById).toHaveBeenCalledWith('test-doc-1');
    }, { timeout: 2000 });
  });

  test('retry functionality works with documents without OCR text', async () => {
    const user = userEvent.setup();
    
    // Mock document without OCR text
    mockGetById.mockResolvedValue({
      data: {
        ...mockDocument,
        has_ocr_text: false,
      },
    });

    renderDocumentDetailsPage();

    await waitFor(() => {
      expect(screen.getByText('Document Details')).toBeInTheDocument();
    });

    // Retry button should still be available
    const retryButton = screen.getByText('Retry OCR');
    expect(retryButton).toBeInTheDocument();

    await user.click(retryButton);

    await waitFor(() => {
      expect(mockBulkRetryOcr).toHaveBeenCalledWith({
        mode: 'specific',
        document_ids: ['test-doc-1'],
        priority_override: 15,
      });
    });
  });

  test('retry history modal receives correct props', async () => {
    const user = userEvent.setup();
    renderDocumentDetailsPage();

    await waitFor(() => {
      expect(screen.getByText('Document Details')).toBeInTheDocument();
    });

    const historyButton = screen.getByText('Retry History');
    await user.click(historyButton);

    // Verify modal props are passed correctly
    expect(screen.getByText('Document ID: test-doc-1')).toBeInTheDocument();
    expect(screen.getByText('Retry History for test-document.pdf')).toBeInTheDocument();
  });
});