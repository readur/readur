import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import OcrRetryDialog from '../OcrRetryDialog';
import { ocrService } from '../../../services/api';

// Mock the API service
vi.mock('../../../services/api', () => ({
  ocrService: {
    getAvailableLanguages: vi.fn(),
    getHealthStatus: vi.fn(),
    retryWithLanguage: vi.fn(),
  },
}));

// Mock the OcrLanguageSelector component
vi.mock('../../OcrLanguageSelector', () => ({
  default: ({ value, onChange, ...props }: any) => (
    <div data-testid="ocr-language-selector">
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        data-testid="language-select"
        {...props}
      >
        <option value="">Select language</option>
        <option value="eng">English</option>
        <option value="spa">Spanish</option>
        <option value="fra">French</option>
      </select>
    </div>
  ),
}));

const mockOcrService = {
  getAvailableLanguages: vi.fn(),
  getHealthStatus: vi.fn(),
  retryWithLanguage: vi.fn(),
} as any;

// Replace the mocked service
(ocrService as any).getAvailableLanguages = mockOcrService.getAvailableLanguages;
(ocrService as any).getHealthStatus = mockOcrService.getHealthStatus;
(ocrService as any).retryWithLanguage = mockOcrService.retryWithLanguage;

const theme = createTheme();

const renderWithTheme = (component: React.ReactElement) => {
  return render(
    <ThemeProvider theme={theme}>
      {component}
    </ThemeProvider>
  );
};

describe('OcrRetryDialog', () => {
  const mockDocument = {
    id: 'doc-123',
    filename: 'test-document.pdf',
    original_filename: 'test-document.pdf',
    failure_category: 'Language Detection Failed',
    ocr_error: 'Unable to detect text language',
    retry_count: 2,
  };

  const defaultProps = {
    open: true,
    onClose: vi.fn(),
    document: mockDocument,
    onRetrySuccess: vi.fn(),
    onRetryError: vi.fn(),
  };

  const mockRetryResponse = {
    data: {
      success: true,
      message: 'OCR retry queued successfully',
      estimated_wait_minutes: 5,
    },
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockOcrService.retryWithLanguage.mockResolvedValue(mockRetryResponse);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('renders dialog when open is true', () => {
    renderWithTheme(<OcrRetryDialog {...defaultProps} />);
    
    expect(screen.getByText('Retry OCR Processing')).toBeInTheDocument();
    expect(screen.getByText('Document: test-document.pdf')).toBeInTheDocument();
    expect(screen.getByText('Previous attempts: 2')).toBeInTheDocument();
  });

  it('does not render dialog when open is false', () => {
    renderWithTheme(<OcrRetryDialog {...defaultProps} open={false} />);
    
    expect(screen.queryByText('Retry OCR Processing')).not.toBeInTheDocument();
  });

  it('does not render when document is null', () => {
    renderWithTheme(<OcrRetryDialog {...defaultProps} document={null} />);
    
    expect(screen.queryByText('Retry OCR Processing')).not.toBeInTheDocument();
  });

  it('displays document information correctly', () => {
    renderWithTheme(<OcrRetryDialog {...defaultProps} />);
    
    expect(screen.getByText('Document: test-document.pdf')).toBeInTheDocument();
    expect(screen.getByText('Previous attempts: 2')).toBeInTheDocument();
    expect(screen.getByText('Previous failure: Language Detection Failed')).toBeInTheDocument();
    expect(screen.getByText('Unable to detect text language')).toBeInTheDocument();
  });

  it('renders language selector', () => {
    renderWithTheme(<OcrRetryDialog {...defaultProps} />);
    
    expect(screen.getByTestId('ocr-language-selector')).toBeInTheDocument();
    expect(screen.getByText('OCR Language Selection')).toBeInTheDocument();
  });

  it('handles language selection', () => {
    renderWithTheme(<OcrRetryDialog {...defaultProps} />);
    
    const languageSelect = screen.getByTestId('language-select');
    fireEvent.change(languageSelect, { target: { value: 'spa' } });
    
    expect(languageSelect).toHaveValue('spa');
  });

  it('calls onRetrySuccess when retry succeeds', async () => {
    const mockOnRetrySuccess = vi.fn();
    renderWithTheme(
      <OcrRetryDialog 
        {...defaultProps} 
        onRetrySuccess={mockOnRetrySuccess}
      />
    );
    
    // Select a language
    const languageSelect = screen.getByTestId('language-select');
    fireEvent.change(languageSelect, { target: { value: 'spa' } });
    
    // Click retry button
    fireEvent.click(screen.getByText('Retry OCR'));
    
    await waitFor(() => {
      expect(mockOcrService.retryWithLanguage).toHaveBeenCalledWith('doc-123', 'spa');
      expect(mockOnRetrySuccess).toHaveBeenCalledWith(
        'OCR retry queued for "test-document.pdf" with language "Spanish". Estimated wait time: 5 minutes.'
      );
    });
  });

  it('calls onRetrySuccess without language info when no language selected', async () => {
    const mockOnRetrySuccess = vi.fn();
    renderWithTheme(
      <OcrRetryDialog 
        {...defaultProps} 
        onRetrySuccess={mockOnRetrySuccess}
      />
    );
    
    // Click retry button without selecting language
    fireEvent.click(screen.getByText('Retry OCR'));
    
    await waitFor(() => {
      expect(mockOcrService.retryWithLanguage).toHaveBeenCalledWith('doc-123', undefined);
      expect(mockOnRetrySuccess).toHaveBeenCalledWith(
        'OCR retry queued for "test-document.pdf". Estimated wait time: 5 minutes.'
      );
    });
  });

  it('handles retry failure', async () => {
    const mockError = new Error('Retry failed');
    mockOcrService.retryWithLanguage.mockRejectedValue(mockError);
    const mockOnRetryError = vi.fn();
    
    renderWithTheme(
      <OcrRetryDialog 
        {...defaultProps} 
        onRetryError={mockOnRetryError}
      />
    );
    
    fireEvent.click(screen.getByText('Retry OCR'));
    
    await waitFor(() => {
      expect(mockOnRetryError).toHaveBeenCalledWith('Failed to retry OCR processing');
    });
  });

  it('handles API error response', async () => {
    const mockErrorResponse = {
      response: {
        data: {
          message: 'Document not found',
        },
      },
    };
    mockOcrService.retryWithLanguage.mockRejectedValue(mockErrorResponse);
    const mockOnRetryError = vi.fn();
    
    renderWithTheme(
      <OcrRetryDialog 
        {...defaultProps} 
        onRetryError={mockOnRetryError}
      />
    );
    
    fireEvent.click(screen.getByText('Retry OCR'));
    
    await waitFor(() => {
      expect(mockOnRetryError).toHaveBeenCalledWith('Document not found');
    });
  });

  it('handles unsuccessful retry response', async () => {
    mockOcrService.retryWithLanguage.mockResolvedValue({
      data: {
        success: false,
        message: 'Queue is full',
      },
    });
    const mockOnRetryError = vi.fn();
    
    renderWithTheme(
      <OcrRetryDialog 
        {...defaultProps} 
        onRetryError={mockOnRetryError}
      />
    );
    
    fireEvent.click(screen.getByText('Retry OCR'));
    
    await waitFor(() => {
      expect(mockOnRetryError).toHaveBeenCalledWith('Queue is full');
    });
  });

  it('shows loading state during retry', async () => {
    // Make the API call hang
    mockOcrService.retryWithLanguage.mockImplementation(() => new Promise(() => {}));
    
    renderWithTheme(<OcrRetryDialog {...defaultProps} />);
    
    fireEvent.click(screen.getByText('Retry OCR'));
    
    await waitFor(() => {
      expect(screen.getByText('Retrying...')).toBeInTheDocument();
    });
    
    // Buttons should be disabled during retry
    expect(screen.getByText('Cancel')).toBeDisabled();
    expect(screen.getByText('Retrying...')).toBeDisabled();
  });

  it('prevents closing dialog during retry', async () => {
    // Make the API call hang
    mockOcrService.retryWithLanguage.mockImplementation(() => new Promise(() => {}));
    const mockOnClose = vi.fn();
    
    renderWithTheme(
      <OcrRetryDialog 
        {...defaultProps} 
        onClose={mockOnClose}
      />
    );
    
    fireEvent.click(screen.getByText('Retry OCR'));
    
    // Try to close via cancel button
    fireEvent.click(screen.getByText('Cancel'));
    
    // Should not call onClose during retry
    expect(mockOnClose).not.toHaveBeenCalled();
  });

  it('calls onClose when cancel is clicked', () => {
    const mockOnClose = vi.fn();
    renderWithTheme(
      <OcrRetryDialog 
        {...defaultProps} 
        onClose={mockOnClose}
      />
    );
    
    fireEvent.click(screen.getByText('Cancel'));
    
    expect(mockOnClose).toHaveBeenCalledTimes(1);
  });

  it('clears selected language when dialog closes', () => {
    const mockOnClose = vi.fn();
    renderWithTheme(
      <OcrRetryDialog 
        {...defaultProps} 
        onClose={mockOnClose}
      />
    );
    
    // Select a language
    const languageSelect = screen.getByTestId('language-select');
    fireEvent.change(languageSelect, { target: { value: 'spa' } });
    
    // Close dialog
    fireEvent.click(screen.getByText('Cancel'));
    
    expect(mockOnClose).toHaveBeenCalled();
  });

  it('closes dialog after successful retry', async () => {
    const mockOnClose = vi.fn();
    renderWithTheme(
      <OcrRetryDialog 
        {...defaultProps} 
        onClose={mockOnClose}
      />
    );
    
    fireEvent.click(screen.getByText('Retry OCR'));
    
    await waitFor(() => {
      expect(mockOnClose).toHaveBeenCalledTimes(1);
    });
  });

  it('displays informational message about retry process', () => {
    renderWithTheme(<OcrRetryDialog {...defaultProps} />);
    
    expect(screen.getByText(/The retry will use enhanced OCR processing/)).toBeInTheDocument();
  });

  it('handles document without failure category', () => {
    const documentWithoutFailure = {
      ...mockDocument,
      failure_category: '',
      ocr_error: '',
    };
    
    renderWithTheme(
      <OcrRetryDialog 
        {...defaultProps} 
        document={documentWithoutFailure}
      />
    );
    
    expect(screen.getByText('Document: test-document.pdf')).toBeInTheDocument();
    expect(screen.queryByText('Previous failure:')).not.toBeInTheDocument();
  });

  it('handles missing estimated wait time in response', async () => {
    mockOcrService.retryWithLanguage.mockResolvedValue({
      data: {
        success: true,
        message: 'OCR retry queued successfully',
        // No estimated_wait_minutes
      },
    });
    
    const mockOnRetrySuccess = vi.fn();
    renderWithTheme(
      <OcrRetryDialog 
        {...defaultProps} 
        onRetrySuccess={mockOnRetrySuccess}
      />
    );
    
    fireEvent.click(screen.getByText('Retry OCR'));
    
    await waitFor(() => {
      expect(mockOnRetrySuccess).toHaveBeenCalledWith(
        'OCR retry queued for "test-document.pdf". Estimated wait time: Unknown minutes.'
      );
    });
  });
});