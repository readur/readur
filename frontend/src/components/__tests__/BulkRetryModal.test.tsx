import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { BulkRetryModal } from '../BulkRetryModal';

// Create unique mock functions for this test file
const { mockBulkRetryOcr, mockGetLanguages, mockRetryWithLanguage } = vi.hoisted(() => ({
  mockBulkRetryOcr: vi.fn(),
  mockGetLanguages: vi.fn(),
  mockRetryWithLanguage: vi.fn(),
}));

// Mock the API module with a unique namespace
vi.mock('../../services/api', async () => {
  const errors = await import('../../services/errors');
  return {
    ErrorHelper: errors.ErrorHelper,
    ErrorCodes: errors.ErrorCodes,
    documentService: {
      bulkRetryOcr: mockBulkRetryOcr,
    },
    ocrService: {
      getAvailableLanguages: mockGetLanguages,
      retryWithLanguage: mockRetryWithLanguage,
    },
  };
});

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
    
    mockGetLanguages.mockResolvedValue({
      data: { available_languages: [{ code: 'eng', name: 'English', installed: true }, { code: 'deu', name: 'German', installed: true }] },
    });
    mockRetryWithLanguage.mockResolvedValue({ data: { success: true } });

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

    expect(screen.getByRole('dialog', { name: 'Bulk OCR retry' })).toBeInTheDocument();
    expect(screen.getByRole('radiogroup', { name: 'Retry mode' })).toBeInTheDocument();
    expect(screen.getByText('Retry all failed OCR documents')).toBeInTheDocument();
    expect(screen.getByText('Retry documents matching criteria')).toBeInTheDocument();
  });

  test('closes modal when close button is clicked', async () => {
    const user = userEvent.setup();
    
    render(<BulkRetryModal {...mockProps} />);

    const closeButton = screen.getByRole('button', { name: 'Cancel' });
    await user.click(closeButton);

    expect(mockProps.onClose).toHaveBeenCalled();
  });

  test('shows preview by default', async () => {
    render(<BulkRetryModal {...mockProps} />);

    const previewButton = screen.getByRole('button', { name: 'Preview' });
    expect(previewButton).toBeInTheDocument();
  });

  test('does not render when modal is closed', async () => {
    render(<BulkRetryModal {...mockProps} open={false} />);

    expect(screen.queryByRole('dialog', { name: 'Bulk OCR retry' })).not.toBeInTheDocument();
  });

  test('resets form when modal is closed and reopened', async () => {
    const { rerender } = render(<BulkRetryModal {...mockProps} open={false} />);

    // Reopen the modal
    rerender(<BulkRetryModal {...mockProps} open={true} />);

    // Should be back to default state
    expect(screen.getByLabelText('Retry all failed OCR documents')).toBeChecked();
  });

  test('keeps Retry disabled until a preview found documents, then retries', async () => {
    const user = userEvent.setup();
    render(<BulkRetryModal {...mockProps} />);
    const retry = screen.getByRole('button', { name: 'Retry 0 documents' });
    expect(retry).toBeDisabled();
    await user.click(screen.getByRole('button', { name: 'Preview' }));
    expect(mockBulkRetryOcr).toHaveBeenCalledWith({ mode: 'all', preview_only: true });
    await user.click(await screen.findByRole('button', { name: 'Retry 5 documents' }));
    expect(mockBulkRetryOcr).toHaveBeenLastCalledWith({ mode: 'all', preview_only: false });
    expect(mockProps.onSuccess).toHaveBeenCalled();
    expect(mockProps.onClose).toHaveBeenCalled();
  });

  test('sends filter criteria in filter mode', async () => {
    const user = userEvent.setup();
    render(<BulkRetryModal {...mockProps} />);
    await user.click(screen.getByLabelText('Retry documents matching criteria'));
    await user.click(screen.getByRole('button', { name: 'PDF' }));
    await user.click(screen.getByRole('button', { name: 'Processing timeout' }));
    await user.click(screen.getByRole('button', { name: '< 5 MB' }));
    await user.click(screen.getByRole('button', { name: 'Preview' }));
    expect(mockBulkRetryOcr).toHaveBeenCalledWith({
      mode: 'filter',
      preview_only: true,
      filter: { mime_types: ['application/pdf'], failure_reasons: ['ocr_timeout'], max_file_size: 5 * 1024 * 1024 },
    });
  });

  test('starts in selected mode with languages available for the selection', async () => {
    render(<BulkRetryModal {...mockProps} selectedDocumentIds={['a', 'b']} />);
    expect(screen.getByLabelText('Retry selected documents (2 selected)')).toBeChecked();
    expect(screen.getByText('Optional. Read the selected documents again with these languages.')).toBeInTheDocument();
  });

  test('shows the server error when the preview fails', async () => {
    mockBulkRetryOcr.mockRejectedValueOnce(new Error('boom'));
    const user = userEvent.setup();
    render(<BulkRetryModal {...mockProps} />);
    await user.click(screen.getByRole('button', { name: 'Preview' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('boom');
  });
});
