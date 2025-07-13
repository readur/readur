import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { BulkRetryModal } from '../BulkRetryModal';

// Create unique mock functions for this test file
const mockBulkRetryOcr = vi.fn();

// Mock the API module with a unique namespace
vi.mock('../../services/api', () => ({
  documentService: {
    bulkRetryOcr: mockBulkRetryOcr,
  },
}));

describe('BulkRetryModal', () => {
  const mockProps = {
    open: true,
    onClose: vi.fn(),
    onSuccess: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.resetAllMocks();
    
    // Reset mock props
    mockProps.onClose.mockClear();
    mockProps.onSuccess.mockClear();
    
    // Default mock response
    mockBulkRetryOcr.mockResolvedValue({
      data: {
        success: true,
        queued_count: 5,
        matched_count: 5,
        documents: [],
        estimated_total_time_minutes: 2.5,
        message: 'Operation completed successfully',
      },
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
    vi.resetAllMocks();
  });

  test('renders modal with title and form elements', async () => {
    render(<BulkRetryModal {...mockProps} />);

    expect(screen.getByText('Bulk OCR Retry')).toBeInTheDocument();
    expect(screen.getByText('Retry Mode')).toBeInTheDocument();
    expect(screen.getByText('Retry all failed OCR documents')).toBeInTheDocument();
    expect(screen.getByText('Retry documents matching criteria')).toBeInTheDocument();
  });

  test('closes modal when close button is clicked', async () => {
    const user = userEvent.setup();
    
    render(<BulkRetryModal {...mockProps} />);

    const closeButton = screen.getByText('Cancel');
    await user.click(closeButton);

    expect(mockProps.onClose).toHaveBeenCalled();
  });

  test('shows preview by default', async () => {
    render(<BulkRetryModal {...mockProps} />);

    const previewButton = screen.getByText('Preview');
    expect(previewButton).toBeInTheDocument();
  });

  test('does not render when modal is closed', async () => {
    render(<BulkRetryModal {...mockProps} open={false} />);

    expect(screen.queryByText('Bulk OCR Retry')).not.toBeInTheDocument();
  });

  test('resets form when modal is closed and reopened', async () => {
    const { rerender } = render(<BulkRetryModal {...mockProps} open={false} />);

    // Reopen the modal
    rerender(<BulkRetryModal {...mockProps} open={true} />);

    // Should be back to default state
    expect(screen.getByLabelText('Retry all failed OCR documents')).toBeChecked();
  });
});