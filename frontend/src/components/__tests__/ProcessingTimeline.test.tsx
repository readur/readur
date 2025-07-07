import React from 'react';
import { render, screen } from '@testing-library/react';
import { ThemeProvider } from '@mui/material/styles';
import theme from '../../theme';
import ProcessingTimeline from '../ProcessingTimeline';

const renderWithTheme = (component: React.ReactElement) => {
  return render(
    <ThemeProvider theme={theme}>
      {component}
    </ThemeProvider>
  );
};

describe('ProcessingTimeline', () => {
  const mockProps = {
    documentId: 'doc-123',
    fileName: 'test-document.pdf',
    createdAt: '2024-01-01T12:00:00Z',
    updatedAt: '2024-01-01T12:30:00Z',
    userId: 'user-123',
    ocrStatus: 'completed',
    ocrCompletedAt: '2024-01-01T12:15:00Z',
    ocrRetryCount: 0,
  };

  it('renders processing timeline', () => {
    renderWithTheme(
      <ProcessingTimeline {...mockProps} />
    );

    expect(screen.getByText('Processing Timeline')).toBeInTheDocument();
    expect(screen.getByText('Document Uploaded')).toBeInTheDocument();
    expect(screen.getByText('OCR Processing Completed')).toBeInTheDocument();
  });

  it('shows retry information when retries exist', () => {
    renderWithTheme(
      <ProcessingTimeline 
        {...mockProps} 
        ocrRetryCount={2}
      />
    );

    expect(screen.getByText('2 retries')).toBeInTheDocument();
    expect(screen.getByText('Detailed Retry History')).toBeInTheDocument();
  });

  it('renders compact view correctly', () => {
    renderWithTheme(
      <ProcessingTimeline {...mockProps} compact={true} />
    );

    expect(screen.getByText('Processing Timeline')).toBeInTheDocument();
    expect(screen.getByText('View Full Timeline')).toBeInTheDocument();
  });

  it('handles OCR error status', () => {
    renderWithTheme(
      <ProcessingTimeline 
        {...mockProps} 
        ocrStatus="failed"
        ocrError="OCR processing failed due to low image quality"
      />
    );

    expect(screen.getByText('OCR Processing Failed')).toBeInTheDocument();
  });

  it('shows pending OCR status', () => {
    renderWithTheme(
      <ProcessingTimeline 
        {...mockProps} 
        ocrStatus="processing"
        ocrCompletedAt={undefined}
      />
    );

    expect(screen.getByText('OCR Processing Started')).toBeInTheDocument();
  });

  it('displays event count', () => {
    renderWithTheme(
      <ProcessingTimeline {...mockProps} />
    );

    // Should show at least 2 events (upload + OCR completion)
    expect(screen.getByText(/\d+ events/)).toBeInTheDocument();
  });
});