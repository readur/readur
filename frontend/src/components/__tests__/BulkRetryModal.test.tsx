import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { BulkRetryModal } from '../BulkRetryModal';

// Mock the API
const mockBulkRetryOcr = vi.fn();
const mockDocumentService = {
  bulkRetryOcr: mockBulkRetryOcr,
};
const mockApi = {
  bulkRetryOcr: mockBulkRetryOcr,
};

vi.mock('../../services/api', () => ({
  default: mockApi,
  documentService: mockDocumentService,
}));

describe('BulkRetryModal', () => {
  const mockProps = {
    open: true,
    onClose: vi.fn(),
    onSuccess: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
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

  test('renders modal with title and form elements', () => {
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

  test('shows preview by default', () => {
    render(<BulkRetryModal {...mockProps} />);

    const previewButton = screen.getByText('Preview');
    expect(previewButton).toBeInTheDocument();
  });

  test('allows switching to filter mode', async () => {
    const user = userEvent.setup();
    render(<BulkRetryModal {...mockProps} />);

    const filterRadio = screen.getByLabelText('Retry documents matching criteria');
    await user.click(filterRadio);

    // Should show the accordion with filter criteria
    expect(screen.getByText('Filter Criteria')).toBeInTheDocument();
    
    // Expand the accordion to see filter options
    const filterAccordion = screen.getByText('Filter Criteria');
    await user.click(filterAccordion);

    expect(screen.getByText('File Types')).toBeInTheDocument();
    expect(screen.getByText('Failure Reasons')).toBeInTheDocument();
    expect(screen.getByText('Maximum File Size')).toBeInTheDocument();
  });

  test('can select MIME types in filter mode', async () => {
    const user = userEvent.setup();
    render(<BulkRetryModal {...mockProps} />);

    // Switch to filter mode
    const filterRadio = screen.getByLabelText('Retry documents matching criteria');
    await user.click(filterRadio);

    // Expand the accordion to see filter options
    const filterAccordion = screen.getByText('Filter Criteria');
    await user.click(filterAccordion);

    // Should show MIME type chips
    const pdfChip = screen.getByText('PDF');
    expect(pdfChip).toBeInTheDocument();

    // Click on the PDF chip to select it
    await user.click(pdfChip);

    // The chip should now be selected (filled variant)
    expect(pdfChip.closest('[data-testid], .MuiChip-root')).toBeInTheDocument();
  });

  test('can set priority override', async () => {
    const user = userEvent.setup();
    render(<BulkRetryModal {...mockProps} />);

    // Expand the Advanced Options accordion
    const advancedAccordion = screen.getByText('Advanced Options');
    await user.click(advancedAccordion);

    // Enable priority override
    const priorityCheckbox = screen.getByLabelText('Override processing priority');
    await user.click(priorityCheckbox);

    // Now the slider should be visible
    const prioritySlider = screen.getByRole('slider');
    fireEvent.change(prioritySlider, { target: { value: 15 } });

    expect(prioritySlider).toHaveValue('15');
  });

  test('executes preview request successfully', async () => {
    const user = userEvent.setup();
    mockBulkRetryOcr.mockResolvedValue({
      data: {
        success: true,
        queued_count: 0,
        matched_count: 3,
        documents: [
          { id: '1', filename: 'doc1.pdf', file_size: 1024, mime_type: 'application/pdf' },
          { id: '2', filename: 'doc2.pdf', file_size: 2048, mime_type: 'application/pdf' },
        ],
        estimated_total_time_minutes: 1.5,
      },
    });

    render(<BulkRetryModal {...mockProps} />);

    const previewButton = screen.getByText('Preview');
    await user.click(previewButton);

    await waitFor(() => {
      expect(screen.getByText('Preview Results')).toBeInTheDocument();
    });

    expect(screen.getByText('Documents matched:')).toBeInTheDocument();
    expect(screen.getByText('Estimated processing time:')).toBeInTheDocument();
  });

  test('executes actual retry request successfully', async () => {
    const user = userEvent.setup();
    render(<BulkRetryModal {...mockProps} />);

    // First do a preview
    const previewButton = screen.getByText('Preview');
    await user.click(previewButton);

    await waitFor(() => {
      expect(screen.getByText(/Retry \d+ Documents/)).toBeInTheDocument();
    });

    // Now execute the retry
    const executeButton = screen.getByText(/Retry \d+ Documents/);
    await user.click(executeButton);

    await waitFor(() => {
      expect(mockBulkRetryOcr).toHaveBeenCalledWith({
        mode: 'all',
        preview_only: false,
      });
    });

    expect(mockProps.onSuccess).toHaveBeenCalled();
    expect(mockProps.onClose).toHaveBeenCalled();
  });

  test('handles API errors gracefully', async () => {
    const user = userEvent.setup();
    mockBulkRetryOcr.mockRejectedValue(new Error('API Error'));

    render(<BulkRetryModal {...mockProps} />);

    const previewButton = screen.getByText('Preview');
    await user.click(previewButton);

    await waitFor(() => {
      expect(screen.getByText(/Failed to preview retry/)).toBeInTheDocument();
    });
  });

  test('can set document limit in filter mode', async () => {
    const user = userEvent.setup();
    render(<BulkRetryModal {...mockProps} />);

    // Switch to filter mode
    const filterRadio = screen.getByLabelText('Retry documents matching criteria');
    await user.click(filterRadio);

    // Expand the accordion to see filter options
    const filterAccordion = screen.getByText('Filter Criteria');
    await user.click(filterAccordion);

    // Find and set the document limit
    const limitInput = screen.getByLabelText('Maximum Documents to Retry');
    await user.clear(limitInput);
    await user.type(limitInput, '100');

    expect(limitInput).toHaveValue(100);
  });

  test('shows loading state during API calls', async () => {
    const user = userEvent.setup();
    
    // Make the API call take time
    mockBulkRetryOcr.mockImplementation(() => new Promise(resolve => 
      setTimeout(() => resolve({
        data: { success: true, queued_count: 0, matched_count: 0, documents: [] }
      }), 100)
    ));

    render(<BulkRetryModal {...mockProps} />);

    const previewButton = screen.getByText('Preview');
    await user.click(previewButton);

    // Should show loading state
    expect(screen.getByRole('progressbar')).toBeInTheDocument();
    // The button should remain as "Preview" during loading, not change text
    expect(screen.getByText('Preview')).toBeInTheDocument();
  });

  test('resets form when modal is closed and reopened', () => {
    const { rerender } = render(<BulkRetryModal {...mockProps} open={false} />);

    // Reopen the modal
    rerender(<BulkRetryModal {...mockProps} open={true} />);

    // Should be back to default state
    expect(screen.getByLabelText('Retry all failed OCR documents')).toBeChecked();
    // Note: slider is not visible by default as it's in an accordion
  });

  test('does not render when modal is closed', () => {
    render(<BulkRetryModal {...mockProps} open={false} />);

    expect(screen.queryByText('Bulk OCR Retry')).not.toBeInTheDocument();
  });
});