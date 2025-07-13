import React from 'react';
import { render, screen } from '@testing-library/react';
import { ThemeProvider } from '@mui/material/styles';
import theme from '../../theme';
import MetadataParser from '../MetadataParser';

const renderWithTheme = (component: React.ReactElement) => {
  return render(
    <ThemeProvider theme={theme}>
      {component}
    </ThemeProvider>
  );
};

describe('MetadataParser', () => {
  const mockImageMetadata = {
    exif: {
      make: 'Canon',
      model: 'EOS R5',
      focal_length: 85,
      aperture: 2.8,
      iso: 800,
      width: 4096,
      height: 2048,
      date_time_original: '2024-01-01T12:00:00Z',
    },
  };

  const mockPdfMetadata = {
    pdf: {
      title: 'Sample Document',
      author: 'Test Author',
      creator: 'Adobe Acrobat',
      page_count: 5,
      creation_date: '2024-01-01T12:00:00Z',
    },
  };

  it('renders EXIF data for image files', () => {
    renderWithTheme(
      <MetadataParser 
        metadata={mockImageMetadata}
        fileType="image/jpeg"
      />
    );

    expect(screen.getByText('Camera')).toBeInTheDocument();
    expect(screen.getByText('Canon')).toBeInTheDocument();
    expect(screen.getByText('EOS R5')).toBeInTheDocument();
  });

  it('renders PDF metadata for PDF files', () => {
    renderWithTheme(
      <MetadataParser 
        metadata={mockPdfMetadata}
        fileType="application/pdf"
      />
    );

    expect(screen.getByText('Document Info')).toBeInTheDocument();
    expect(screen.getByText('Sample Document')).toBeInTheDocument();
    expect(screen.getByText('Test Author')).toBeInTheDocument();
  });

  it('renders compact view correctly', () => {
    renderWithTheme(
      <MetadataParser 
        metadata={mockImageMetadata}
        fileType="image/jpeg"
        compact={true}
      />
    );

    expect(screen.getByText('Camera')).toBeInTheDocument();
    // In compact mode, should show limited items
    expect(screen.getByText('Canon')).toBeInTheDocument();
  });

  it('shows message when no metadata available', () => {
    renderWithTheme(
      <MetadataParser 
        metadata={{}}
        fileType="text/plain"
      />
    );

    expect(screen.getByText('No detailed metadata available for this file type')).toBeInTheDocument();
  });
});