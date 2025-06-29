import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import OcrLanguageSelector from '../OcrLanguageSelector';
import { ocrService } from '../../../services/api';

// Mock the API service
vi.mock('../../../services/api', () => ({
  ocrService: {
    getAvailableLanguages: vi.fn(),
  },
}));

const mockOcrService = vi.mocked(ocrService);

const theme = createTheme();

const renderWithTheme = (component: React.ReactElement) => {
  return render(
    <ThemeProvider theme={theme}>
      {component}
    </ThemeProvider>
  );
};

describe('OcrLanguageSelector', () => {
  const defaultProps = {
    value: 'eng',
    onChange: vi.fn(),
    label: 'OCR Language',
  };

  const mockLanguagesResponse = {
    data: {
      languages: [
        { code: 'eng', name: 'English' },
        { code: 'spa', name: 'Spanish' },
        { code: 'fra', name: 'French' },
        { code: 'deu', name: 'German' },
      ],
      current_user_language: 'eng',
    },
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockOcrService.getAvailableLanguages.mockResolvedValue(mockLanguagesResponse);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('renders with default props', async () => {
    renderWithTheme(<OcrLanguageSelector {...defaultProps} />);
    
    expect(screen.getByLabelText('OCR Language')).toBeInTheDocument();
    
    // Wait for languages to load
    await waitFor(() => {
      expect(mockOcrService.getAvailableLanguages).toHaveBeenCalledTimes(1);
    });
  });

  it('displays loading state initially', () => {
    renderWithTheme(<OcrLanguageSelector {...defaultProps} />);
    
    expect(screen.getByTestId('loading-languages')).toBeInTheDocument();
  });

  it('loads and displays available languages', async () => {
    renderWithTheme(<OcrLanguageSelector {...defaultProps} />);
    
    await waitFor(() => {
      expect(mockOcrService.getAvailableLanguages).toHaveBeenCalledTimes(1);
    });

    // Open the select dropdown
    fireEvent.mouseDown(screen.getByRole('combobox'));
    
    await waitFor(() => {
      expect(screen.getByText('English')).toBeInTheDocument();
      expect(screen.getByText('Spanish')).toBeInTheDocument();
      expect(screen.getByText('French')).toBeInTheDocument();
      expect(screen.getByText('German')).toBeInTheDocument();
    });
  });

  it('shows current language indicator when enabled', async () => {
    renderWithTheme(
      <OcrLanguageSelector 
        {...defaultProps} 
        showCurrentIndicator={true}
      />
    );
    
    await waitFor(() => {
      expect(mockOcrService.getAvailableLanguages).toHaveBeenCalledTimes(1);
    });

    // Open the select dropdown
    fireEvent.mouseDown(screen.getByRole('combobox'));
    
    await waitFor(() => {
      expect(screen.getByText('(Current)')).toBeInTheDocument();
    });
  });

  it('calls onChange when language is selected', async () => {
    const mockOnChange = vi.fn();
    renderWithTheme(
      <OcrLanguageSelector 
        {...defaultProps} 
        onChange={mockOnChange}
      />
    );
    
    await waitFor(() => {
      expect(mockOcrService.getAvailableLanguages).toHaveBeenCalledTimes(1);
    });

    // Open the select dropdown
    fireEvent.mouseDown(screen.getByRole('combobox'));
    
    // Select Spanish
    fireEvent.click(screen.getByText('Spanish'));
    
    expect(mockOnChange).toHaveBeenCalledWith('spa');
  });

  it('displays error state when API call fails', async () => {
    const mockError = new Error('Failed to fetch languages');
    mockOcrService.getAvailableLanguages.mockRejectedValue(mockError);
    
    renderWithTheme(<OcrLanguageSelector {...defaultProps} />);
    
    await waitFor(() => {
      expect(screen.getByText('Failed to load languages')).toBeInTheDocument();
    });
  });

  it('retries loading languages when retry button is clicked', async () => {
    const mockError = new Error('Failed to fetch languages');
    mockOcrService.getAvailableLanguages.mockRejectedValueOnce(mockError);
    mockOcrService.getAvailableLanguages.mockResolvedValueOnce(mockLanguagesResponse);
    
    renderWithTheme(<OcrLanguageSelector {...defaultProps} />);
    
    // Wait for error state
    await waitFor(() => {
      expect(screen.getByText('Failed to load languages')).toBeInTheDocument();
    });
    
    // Click retry button
    fireEvent.click(screen.getByText('Retry'));
    
    // Should call API again
    await waitFor(() => {
      expect(mockOcrService.getAvailableLanguages).toHaveBeenCalledTimes(2);
    });
  });

  it('renders with custom label', () => {
    renderWithTheme(
      <OcrLanguageSelector 
        {...defaultProps} 
        label="Custom Language Label"
      />
    );
    
    expect(screen.getByLabelText('Custom Language Label')).toBeInTheDocument();
  });

  it('renders with helper text', () => {
    renderWithTheme(
      <OcrLanguageSelector 
        {...defaultProps} 
        helperText="Choose your preferred language"
      />
    );
    
    expect(screen.getByText('Choose your preferred language')).toBeInTheDocument();
  });

  it('respects size prop', () => {
    renderWithTheme(
      <OcrLanguageSelector 
        {...defaultProps} 
        size="small"
      />
    );
    
    const select = screen.getByRole('combobox');
    expect(select).toHaveClass('MuiInputBase-sizeSmall');
  });

  it('respects disabled prop', () => {
    renderWithTheme(
      <OcrLanguageSelector 
        {...defaultProps} 
        disabled={true}
      />
    );
    
    const select = screen.getByRole('combobox');
    expect(select).toBeDisabled();
  });

  it('handles empty language list gracefully', async () => {
    mockOcrService.getAvailableLanguages.mockResolvedValue({
      data: {
        languages: [],
        current_user_language: null,
      },
    });
    
    renderWithTheme(<OcrLanguageSelector {...defaultProps} />);
    
    await waitFor(() => {
      expect(mockOcrService.getAvailableLanguages).toHaveBeenCalledTimes(1);
    });

    // Open the select dropdown
    fireEvent.mouseDown(screen.getByRole('combobox'));
    
    await waitFor(() => {
      expect(screen.getByText('No languages available')).toBeInTheDocument();
    });
  });

  it('displays selected language correctly', async () => {
    renderWithTheme(
      <OcrLanguageSelector 
        {...defaultProps} 
        value="spa"
      />
    );
    
    await waitFor(() => {
      expect(mockOcrService.getAvailableLanguages).toHaveBeenCalledTimes(1);
    });

    // The selected value should be displayed
    expect(screen.getByDisplayValue('spa')).toBeInTheDocument();
  });

  it('handles network errors gracefully', async () => {
    const networkError = new Error('Network Error');
    networkError.name = 'NetworkError';
    mockOcrService.getAvailableLanguages.mockRejectedValue(networkError);
    
    renderWithTheme(<OcrLanguageSelector {...defaultProps} />);
    
    await waitFor(() => {
      expect(screen.getByText('Failed to load languages')).toBeInTheDocument();
      expect(screen.getByText('Check your internet connection')).toBeInTheDocument();
    });
  });

  it('clears selection when value is empty string', async () => {
    renderWithTheme(
      <OcrLanguageSelector 
        {...defaultProps} 
        value=""
      />
    );
    
    await waitFor(() => {
      expect(mockOcrService.getAvailableLanguages).toHaveBeenCalledTimes(1);
    });

    const select = screen.getByRole('combobox');
    expect(select).toHaveValue('');
  });
});