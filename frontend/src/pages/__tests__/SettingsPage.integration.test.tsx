import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import { BrowserRouter } from 'react-router-dom';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import SettingsPage from '../SettingsPage';
import { AuthContext } from '../../contexts/AuthContext';
import api, { ocrService } from '../../services/api';

// Mock the API
vi.mock('../../services/api', () => ({
  default: {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
  },
  ocrService: {
    getAvailableLanguages: vi.fn(),
  },
  queueService: {
    getQueueStats: vi.fn(),
  },
}));

const mockedApi = vi.mocked(api);
const mockedOcrService = vi.mocked(ocrService);

const theme = createTheme();

const mockAuthContext = {
  user: {
    id: 'user-123',
    username: 'testuser',
    email: 'test@example.com',
    created_at: '2023-01-01T00:00:00Z',
  },
  login: vi.fn(),
  logout: vi.fn(),
  loading: false,
};

const renderWithProviders = (component: React.ReactElement) => {
  return render(
    <BrowserRouter>
      <ThemeProvider theme={theme}>
        <AuthContext.Provider value={mockAuthContext}>
          {component}
        </AuthContext.Provider>
      </ThemeProvider>
    </BrowserRouter>
  );
};

describe('Settings Page - OCR Language Integration', () => {
  const mockSettingsResponse = {
    data: {
      ocrLanguage: 'eng',
      concurrentOcrJobs: 2,
      ocrTimeoutSeconds: 300,
      maxFileSizeMb: 50,
      allowedFileTypes: ['pdf', 'png', 'jpg'],
      autoRotateImages: true,
      enableImagePreprocessing: true,
      searchResultsPerPage: 20,
      searchSnippetLength: 200,
      fuzzySearchThreshold: 0.7,
      retentionDays: null,
      enableAutoCleanup: false,
      enableCompression: true,
      memoryLimitMb: 1024,
      cpuPriority: 'normal',
      enableBackgroundOcr: true,
      ocrPageSegmentationMode: 3,
      ocrEngineMode: 3,
      ocrMinConfidence: 30,
      ocrDpi: 300,
      ocrEnhanceContrast: true,
      ocrRemoveNoise: true,
      ocrDetectOrientation: true,
      ocrWhitelistChars: '',
      ocrBlacklistChars: '',
      ocrBrightnessBoost: 0,
      ocrContrastMultiplier: 1.0,
      ocrNoiseReductionLevel: 1,
      ocrSharpeningStrength: 0,
      ocrMorphologicalOperations: false,
      ocrAdaptiveThresholdWindowSize: 15,
      ocrHistogramEqualization: false,
      ocrUpscaleFactor: 1.0,
      ocrMaxImageWidth: 4000,
      ocrMaxImageHeight: 4000,
      saveProcessedImages: false,
      ocrQualityThresholdBrightness: 50,
      ocrQualityThresholdContrast: 20,
      ocrQualityThresholdNoise: 80,
      ocrQualityThresholdSharpness: 30,
      ocrSkipEnhancement: false,
    },
  };

  const mockLanguagesResponse = {
    data: {
      languages: [
        { code: 'eng', name: 'English' },
        { code: 'spa', name: 'Spanish' },
        { code: 'fra', name: 'French' },
        { code: 'deu', name: 'German' },
        { code: 'ita', name: 'Italian' },
      ],
      current_user_language: 'eng',
    },
  };

  const mockQueueStatsResponse = {
    data: {
      total_jobs: 0,
      pending_jobs: 0,
      processing_jobs: 0,
      completed_jobs: 0,
      failed_jobs: 0,
    },
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockedApi.get.mockImplementation((url) => {
      if (url === '/settings') return Promise.resolve(mockSettingsResponse);
      if (url === '/labels?include_counts=true') return Promise.resolve({ data: [] });
      return Promise.reject(new Error(`Unexpected GET request to ${url}`));
    });
    mockedOcrService.getAvailableLanguages.mockResolvedValue(mockLanguagesResponse);
    vi.mocked(require('../../services/api').queueService.getQueueStats).mockResolvedValue(mockQueueStatsResponse);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('loads and displays current OCR language in settings', async () => {
    renderWithProviders(<SettingsPage />);

    await waitFor(() => {
      expect(mockedApi.get).toHaveBeenCalledWith('/settings');
      expect(mockedOcrService.getAvailableLanguages).toHaveBeenCalled();
    });

    // Should display the OCR language selector
    expect(screen.getByText('OCR Language')).toBeInTheDocument();
  });

  it('successfully changes OCR language and saves settings', async () => {
    const mockUpdateResponse = { data: { success: true } };
    mockedApi.put.mockResolvedValueOnce(mockUpdateResponse);

    renderWithProviders(<SettingsPage />);

    // Wait for page to load
    await waitFor(() => {
      expect(screen.getByText('OCR Language')).toBeInTheDocument();
    });

    // Find and open the language selector
    const languageSelector = screen.getByLabelText('OCR Language');
    fireEvent.mouseDown(languageSelector);

    // Wait for dropdown options to appear
    await waitFor(() => {
      expect(screen.getByText('Spanish')).toBeInTheDocument();
    });

    // Select Spanish
    fireEvent.click(screen.getByText('Spanish'));

    // Find and click the save button
    const saveButton = screen.getByText('Save Changes');
    fireEvent.click(saveButton);

    // Verify the API call was made with updated settings
    await waitFor(() => {
      expect(mockedApi.put).toHaveBeenCalledWith('/settings', {
        ...mockSettingsResponse.data,
        ocrLanguage: 'spa',
      });
    });

    // Should show success message
    await waitFor(() => {
      expect(screen.getByText('Settings saved successfully')).toBeInTheDocument();
    });
  });

  it('handles OCR language loading errors gracefully', async () => {
    mockedOcrService.getAvailableLanguages.mockRejectedValueOnce(new Error('Failed to load languages'));

    renderWithProviders(<SettingsPage />);

    await waitFor(() => {
      expect(mockedOcrService.getAvailableLanguages).toHaveBeenCalled();
    });

    // Should still render the page but with error state in language selector
    expect(screen.getByText('OCR Language')).toBeInTheDocument();
  });

  it('handles settings save errors appropriately', async () => {
    const mockError = new Error('Failed to save settings');
    mockedApi.put.mockRejectedValueOnce(mockError);

    renderWithProviders(<SettingsPage />);

    // Wait for page to load
    await waitFor(() => {
      expect(screen.getByText('OCR Language')).toBeInTheDocument();
    });

    // Change a setting
    const languageSelector = screen.getByLabelText('OCR Language');
    fireEvent.mouseDown(languageSelector);

    await waitFor(() => {
      expect(screen.getByText('French')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('French'));

    // Try to save
    const saveButton = screen.getByText('Save Changes');
    fireEvent.click(saveButton);

    // Should show error message
    await waitFor(() => {
      expect(screen.getByText(/Failed to save settings/)).toBeInTheDocument();
    });
  });

  it('preserves other settings when changing OCR language', async () => {
    const mockUpdateResponse = { data: { success: true } };
    mockedApi.put.mockResolvedValueOnce(mockUpdateResponse);

    renderWithProviders(<SettingsPage />);

    // Wait for page to load
    await waitFor(() => {
      expect(screen.getByText('OCR Language')).toBeInTheDocument();
    });

    // Change OCR language
    const languageSelector = screen.getByLabelText('OCR Language');
    fireEvent.mouseDown(languageSelector);

    await waitFor(() => {
      expect(screen.getByText('German')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('German'));

    // Save settings
    const saveButton = screen.getByText('Save Changes');
    fireEvent.click(saveButton);

    // Verify all original settings are preserved except OCR language
    await waitFor(() => {
      expect(mockedApi.put).toHaveBeenCalledWith('/settings', {
        ...mockSettingsResponse.data,
        ocrLanguage: 'deu',
      });
    });
  });

  it('shows loading state while fetching languages', async () => {
    // Make the language fetch hang
    mockedOcrService.getAvailableLanguages.mockImplementation(() => new Promise(() => {}));

    renderWithProviders(<SettingsPage />);

    await waitFor(() => {
      expect(screen.getByText('OCR Language')).toBeInTheDocument();
    });

    // Should show loading indicator in the language selector
    expect(screen.getByTestId('loading-languages')).toBeInTheDocument();
  });

  it('handles empty language list', async () => {
    mockedOcrService.getAvailableLanguages.mockResolvedValueOnce({
      data: {
        languages: [],
        current_user_language: null,
      },
    });

    renderWithProviders(<SettingsPage />);

    await waitFor(() => {
      expect(mockedOcrService.getAvailableLanguages).toHaveBeenCalled();
    });

    // Should still render the language selector
    expect(screen.getByText('OCR Language')).toBeInTheDocument();

    // Open the dropdown
    const languageSelector = screen.getByLabelText('OCR Language');
    fireEvent.mouseDown(languageSelector);

    // Should show "No languages available"
    await waitFor(() => {
      expect(screen.getByText('No languages available')).toBeInTheDocument();
    });
  });

  it('indicates current user language in the dropdown', async () => {
    renderWithProviders(<SettingsPage />);

    await waitFor(() => {
      expect(screen.getByText('OCR Language')).toBeInTheDocument();
    });

    // Open the language selector
    const languageSelector = screen.getByLabelText('OCR Language');
    fireEvent.mouseDown(languageSelector);

    // Should show current language indicator
    await waitFor(() => {
      expect(screen.getByText('(Current)')).toBeInTheDocument();
    });
  });

  it('updates language selector when settings are reloaded', async () => {
    const { rerender } = renderWithProviders(<SettingsPage />);

    // Initial load
    await waitFor(() => {
      expect(screen.getByText('OCR Language')).toBeInTheDocument();
    });

    // Update mock to return different language
    const updatedSettingsResponse = {
      ...mockSettingsResponse,
      data: {
        ...mockSettingsResponse.data,
        ocrLanguage: 'spa',
      },
    };

    mockedApi.get.mockImplementation((url) => {
      if (url === '/settings') return Promise.resolve(updatedSettingsResponse);
      if (url === '/labels?include_counts=true') return Promise.resolve({ data: [] });
      return Promise.reject(new Error(`Unexpected GET request to ${url}`));
    });

    // Rerender component
    rerender(
      <BrowserRouter>
        <ThemeProvider theme={theme}>
          <AuthContext.Provider value={mockAuthContext}>
            <SettingsPage />
          </AuthContext.Provider>
        </ThemeProvider>
      </BrowserRouter>
    );

    // Should reflect the updated language
    await waitFor(() => {
      const languageSelector = screen.getByLabelText('OCR Language');
      expect(languageSelector).toHaveValue('spa');
    });
  });
});