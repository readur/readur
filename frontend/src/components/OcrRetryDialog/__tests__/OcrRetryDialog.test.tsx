import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { vi, describe, it, expect, beforeEach } from 'vitest';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import OcrRetryDialog from '../OcrRetryDialog';

// Mock the API service completely to prevent network calls
vi.mock('../../../services/api', () => ({
  ocrService: {
    getAvailableLanguages: vi.fn(),
    getHealthStatus: vi.fn(), 
    retryWithLanguage: vi.fn(),
  },
}));

// Mock the OcrLanguageSelector to prevent API calls
vi.mock('../OcrLanguageSelector', () => ({
  default: () => <div data-testid="ocr-language-selector">Mock Language Selector</div>
}));

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

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders dialog when open is true', () => {
    renderWithTheme(<OcrRetryDialog {...defaultProps} />);
    
    expect(screen.getByText('Retry OCR Processing')).toBeInTheDocument();
    expect(screen.getByText(/Document.*test-document\.pdf/)).toBeInTheDocument();
    expect(screen.getByText(/Previous attempts.*2/)).toBeInTheDocument();
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
    
    expect(screen.getByText(/Document.*test-document\.pdf/)).toBeInTheDocument();
    expect(screen.getByText(/Previous attempts.*2/)).toBeInTheDocument();
    expect(screen.getByText(/Language Detection Failed/)).toBeInTheDocument();
    expect(screen.getByText(/Unable to detect text language/)).toBeInTheDocument();
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
    
    expect(screen.getByText(/Document.*test-document\.pdf/)).toBeInTheDocument();
    expect(screen.queryByText(/Previous failure/)).not.toBeInTheDocument();
  });

  it('displays retry and cancel buttons', () => {
    renderWithTheme(<OcrRetryDialog {...defaultProps} />);
    
    expect(screen.getByText('Retry OCR')).toBeInTheDocument();
    expect(screen.getByText('Cancel')).toBeInTheDocument();
  });
});