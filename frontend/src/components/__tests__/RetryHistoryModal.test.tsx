import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { RetryHistoryModal } from '../RetryHistoryModal';

// Create unique mock functions for this test file
const { mockGetDocumentRetryHistory } = vi.hoisted(() => ({ mockGetDocumentRetryHistory: vi.fn() }));

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

    expect(screen.queryByRole('dialog', { name: 'OCR retry history' })).not.toBeInTheDocument();
  });

  test('renders modal with correct structure when open', async () => {
    render(<RetryHistoryModal {...mockProps} />);

    // Check that the modal renders with the correct title
    expect(screen.getByRole('dialog', { name: 'OCR retry history' })).toBeInTheDocument();
    expect(screen.getByText('test-document.pdf')).toBeInTheDocument();
  });

  test('handles missing documentName gracefully', async () => {
    render(<RetryHistoryModal {...mockProps} documentName={undefined} />);

    // The component only shows documentName if it exists, so we just check the modal title appears
    expect(screen.getByRole('dialog', { name: 'OCR retry history' })).toBeInTheDocument();
  });

  test('lists retry attempts with reason, priority and queue state', async () => {
    mockGetDocumentRetryHistory.mockResolvedValue({
      data: {
        document_id: 'test-doc-123',
        total_retries: 2,
        retry_history: [
          { id: 'r1', retry_reason: 'bulk_retry_all', priority: 16, queue_id: '12345678abcd', created_at: '2026-04-01T10:00:00Z', previous_status: 'failed', previous_failure_reason: 'ocr_timeout' },
          { id: 'r2', retry_reason: 'custom_reason', priority: 3, created_at: '2026-03-01T10:00:00Z' },
        ],
      },
    });
    render(<RetryHistoryModal {...mockProps} />);
    const grid = await screen.findByRole('grid', { name: 'OCR retry history' });
    const rows = within(grid).getAllByRole('row').slice(1);
    expect(rows[0]).toHaveTextContent('Bulk retry (all)');
    expect(rows[0]).toHaveTextContent('Very high (16)');
    expect(rows[0]).toHaveTextContent('Queued · 12345678');
    expect(rows[0]).toHaveTextContent('failed · ocr timeout');
    expect(rows[1]).toHaveTextContent('custom reason');
    expect(rows[1]).toHaveTextContent('Very low (3)');
    expect(rows[1]).toHaveTextContent('Not queued');
    expect(screen.getByText('2 retry attempts')).toBeInTheDocument();
  });

  test('says so when there are no attempts, and closes', async () => {
    const user = userEvent.setup();
    render(<RetryHistoryModal {...mockProps} />);
    expect(await screen.findByText('No retry attempts found for this document.')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Close' }));
    expect(mockProps.onClose).toHaveBeenCalled();
  });
});
