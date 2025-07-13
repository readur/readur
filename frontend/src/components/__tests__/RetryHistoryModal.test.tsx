import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { RetryHistoryModal } from '../RetryHistoryModal';

// Create unique mock functions for this test file
const mockGetDocumentRetryHistory = vi.fn();

// Mock the API module with a unique namespace for this test
vi.mock('../../services/api', () => ({
  documentService: {
    getDocumentRetryHistory: mockGetDocumentRetryHistory,
  },
}));

describe('RetryHistoryModal', () => {
  const mockProps = {
    open: true,
    onClose: vi.fn(),
    documentId: 'test-doc-123',
    documentName: 'test-document.pdf',
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.resetAllMocks();
    
    // Reset mock props
    mockProps.onClose.mockClear();
    
    // Default mock response
    mockGetDocumentRetryHistory.mockResolvedValue({
      data: {
        document_id: 'test-doc-123',
        retry_history: [],
        total_retries: 0,
      },
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
    vi.resetAllMocks();
  });

  test('does not render when modal is closed', async () => {
    render(<RetryHistoryModal {...mockProps} open={false} />);

    expect(screen.queryByText('OCR Retry History')).not.toBeInTheDocument();
  });

  test('renders modal with correct structure when open', async () => {
    render(<RetryHistoryModal {...mockProps} />);

    // Check that the modal renders with the correct title
    expect(screen.getByText('OCR Retry History')).toBeInTheDocument();
    expect(screen.getByText('test-document.pdf')).toBeInTheDocument();
  });

  test('handles missing documentName gracefully', async () => {
    render(<RetryHistoryModal {...mockProps} documentName={undefined} />);

    // The component only shows documentName if it exists, so we just check the modal title appears
    expect(screen.getByText('OCR Retry History')).toBeInTheDocument();
  });
});