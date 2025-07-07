import React from 'react';
import { render, screen } from '@testing-library/react';
import { ThemeProvider } from '@mui/material/styles';
import theme from '../../theme';
import FileIntegrityDisplay from '../FileIntegrityDisplay';

const renderWithTheme = (component: React.ReactElement) => {
  return render(
    <ThemeProvider theme={theme}>
      {component}
    </ThemeProvider>
  );
};

describe('FileIntegrityDisplay', () => {
  const mockProps = {
    fileHash: 'a1b2c3d4e5f6789012345678901234567890123456789012345678901234567890',
    fileName: 'test-document.pdf',
    fileSize: 1048576, // 1MB
    mimeType: 'application/pdf',
    createdAt: '2024-01-01T12:00:00Z',
    updatedAt: '2024-01-01T12:00:00Z',
    userId: 'user-123-456-789',
  };

  it('renders file integrity information', () => {
    renderWithTheme(
      <FileIntegrityDisplay {...mockProps} />
    );

    expect(screen.getByText('File Integrity & Verification')).toBeInTheDocument();
    expect(screen.getByText('SHA256 Hash')).toBeInTheDocument();
    expect(screen.getByText('File Properties')).toBeInTheDocument();
  });

  it('displays file hash correctly', () => {
    renderWithTheme(
      <FileIntegrityDisplay {...mockProps} />
    );

    // Should show the full hash in expanded view
    expect(screen.getByText(mockProps.fileHash)).toBeInTheDocument();
  });

  it('shows compact view when compact prop is true', () => {
    renderWithTheme(
      <FileIntegrityDisplay {...mockProps} compact={true} />
    );

    expect(screen.getByText('File Integrity')).toBeInTheDocument();
    // Should show abbreviated hash in compact view
    expect(screen.getByText('a1b2c3d4...34567890')).toBeInTheDocument();
  });

  it('handles missing file hash gracefully', () => {
    renderWithTheme(
      <FileIntegrityDisplay {...mockProps} fileHash={undefined} />
    );

    expect(screen.getByText('Hash not available')).toBeInTheDocument();
    expect(screen.getByText('File hash not available. Enable hash generation in upload settings.')).toBeInTheDocument();
  });

  it('formats file size correctly', () => {
    renderWithTheme(
      <FileIntegrityDisplay {...mockProps} />
    );

    expect(screen.getByText('1 MB')).toBeInTheDocument();
  });

  it('displays user information', () => {
    renderWithTheme(
      <FileIntegrityDisplay {...mockProps} />
    );

    expect(screen.getByText('User: user-123...')).toBeInTheDocument();
  });
});