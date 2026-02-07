import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { AxiosResponse } from 'axios';
import LanguageSelector from '../LanguageSelector';
import { renderWithProviders } from '../../../test/test-utils';
import * as apiModule from '../../../services/api';
import type { AvailableLanguagesResponse } from '../../../services/api';

const mockLanguages = [
  { code: 'eng', name: 'English', installed: true },
  { code: 'spa', name: 'Spanish', installed: true },
  { code: 'fra', name: 'French', installed: true },
  { code: 'deu', name: 'German', installed: true },
  { code: 'ita', name: 'Italian', installed: true },
  { code: 'por', name: 'Portuguese', installed: true },
  { code: 'chi_sim', name: 'Chinese (Simplified)', installed: true },
  { code: 'jpn', name: 'Japanese', installed: true },
  { code: 'ara', name: 'Arabic', installed: true },
  { code: 'tha', name: 'Thai', installed: true },
];

const renderLanguageSelector = async (props: Partial<React.ComponentProps<typeof LanguageSelector>> = {}) => {
  const defaultProps = {
    selectedLanguages: [],
    primaryLanguage: '',
    onLanguagesChange: vi.fn(),
    ...props,
  };

  const result = renderWithProviders(<LanguageSelector {...defaultProps} />);

  return result;
};

describe('LanguageSelector Component', () => {
  let user: ReturnType<typeof userEvent.setup>;
  let mockGetAvailableLanguages: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    // Use vi.spyOn to mock the ocrService.getAvailableLanguages method
    mockGetAvailableLanguages = vi.spyOn(apiModule.ocrService, 'getAvailableLanguages');
    mockGetAvailableLanguages.mockResolvedValue({
      data: { available_languages: mockLanguages },
    } as AxiosResponse<AvailableLanguagesResponse>);
    user = userEvent.setup();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('Basic Rendering', () => {
    test('should render the language selector container', async () => {
      await renderLanguageSelector();
      expect(screen.getByText('OCR Languages')).toBeInTheDocument();
    });

    test('should show default state text when no languages selected', async () => {
      await renderLanguageSelector();
      expect(screen.getByText('No languages selected. Documents will use default OCR language.')).toBeInTheDocument();
    });

    test('should show selection button', async () => {
      await renderLanguageSelector();
      expect(screen.getByText('Select OCR languages...')).toBeInTheDocument();
    });

    test('should show language count when languages are selected', async () => {
      await renderLanguageSelector({
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng'
      });
      expect(screen.getByText('OCR Languages (2/4)')).toBeInTheDocument();
    });

    test('should open dropdown when button is clicked', async () => {
      await renderLanguageSelector();

      await user.click(screen.getByText('Select OCR languages...'));

      await waitFor(() => {
        expect(screen.getByText('Available Languages')).toBeInTheDocument();
      });

      // Wait for loading to complete and languages to appear
      await waitFor(() => {
        expect(screen.getByText('English')).toBeInTheDocument();
      });
      expect(screen.getByText('Spanish')).toBeInTheDocument();
    });

    test('should apply custom className', async () => {
      const { container } = await renderLanguageSelector({ className: 'custom-class' });
      expect(container.firstChild).toHaveClass('custom-class');
    });
  });

  describe('Language Selection', () => {
    test('should show selected languages as tags', async () => {
      await renderLanguageSelector({
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng'
      });

      // Wait for API to load language names
      await waitFor(() => {
        expect(screen.getByText('English')).toBeInTheDocument();
      });
      expect(screen.getByText('Spanish')).toBeInTheDocument();
      expect(screen.getByText('(Primary)')).toBeInTheDocument();
    });

    test('should call onLanguagesChange when language is selected from dropdown', async () => {
      const mockOnChange = vi.fn();
      await renderLanguageSelector({ onLanguagesChange: mockOnChange });

      // Open dropdown
      await user.click(screen.getByText('Select OCR languages...'));

      // Wait for languages to load
      await waitFor(() => {
        expect(screen.getByText('English')).toBeInTheDocument();
      });

      // Select English from the dropdown - click on the language text directly
      await user.click(screen.getByText('English'));

      expect(mockOnChange).toHaveBeenCalledWith(['eng'], 'eng');
    });

    test('should show "Add more languages" when languages are selected', async () => {
      await renderLanguageSelector({
        selectedLanguages: ['eng'],
        primaryLanguage: 'eng'
      });

      expect(screen.getByText('Add more languages (3 remaining)')).toBeInTheDocument();
    });

    test('should handle maximum language limit', async () => {
      await renderLanguageSelector({
        selectedLanguages: ['eng', 'spa', 'fra', 'deu'],
        primaryLanguage: 'eng',
        maxLanguages: 4
      });

      expect(screen.getByText('Add more languages (0 remaining)')).toBeInTheDocument();
    });
  });

  describe('Primary Language', () => {
    test('should show primary language indicator', async () => {
      await renderLanguageSelector({
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng'
      });

      // Wait for API to load language names
      await waitFor(() => {
        expect(screen.getByText('(Primary)')).toBeInTheDocument();
      });
    });

    test('should handle primary language changes', async () => {
      const mockOnChange = vi.fn();
      await renderLanguageSelector({
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng',
        onLanguagesChange: mockOnChange
      });

      // Open dropdown
      await user.click(screen.getByText('Add more languages (2 remaining)'));

      // Wait for languages to load in the dropdown
      await waitFor(() => {
        expect(screen.getByText('Available Languages')).toBeInTheDocument();
      });

      // Find and click the "Set Primary" button for Spanish (the non-primary language)
      const setPrimaryButton = screen.getByRole('button', { name: 'Set Primary' });
      await user.click(setPrimaryButton);

      // Verify onLanguagesChange was called with the same languages but Spanish as primary
      expect(mockOnChange).toHaveBeenCalledWith(['eng', 'spa'], 'spa');
    });
  });

  describe('Disabled State', () => {
    test('should not show button when disabled', async () => {
      await renderLanguageSelector({ disabled: true });

      expect(screen.queryByText('Select OCR languages...')).not.toBeInTheDocument();
    });

    test('should not show remove buttons when disabled', async () => {
      await renderLanguageSelector({
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng',
        disabled: true
      });

      // Wait for API to load language names
      await waitFor(() => {
        expect(screen.getByText('English')).toBeInTheDocument();
      });
      expect(screen.getByText('Spanish')).toBeInTheDocument();

      // Verify that delete icons (XMarkIcon) are not present in the chips
      // When disabled, onDelete is not passed to Chip, so no delete icons should render
      const deleteIcons = document.querySelectorAll('.MuiChip-deleteIcon');
      expect(deleteIcons).toHaveLength(0);
    });
  });

  describe('Custom Configuration', () => {
    test('should respect custom maxLanguages prop', async () => {
      await renderLanguageSelector({
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng',
        maxLanguages: 3
      });

      expect(screen.getByText('OCR Languages (2/3)')).toBeInTheDocument();
      expect(screen.getByText('Add more languages (1 remaining)')).toBeInTheDocument();
    });

    test('should handle edge case of maxLanguages = 1', async () => {
      await renderLanguageSelector({
        selectedLanguages: ['eng'],
        primaryLanguage: 'eng',
        maxLanguages: 1
      });

      expect(screen.getByText('OCR Languages (1/1)')).toBeInTheDocument();
      expect(screen.getByText('Add more languages (0 remaining)')).toBeInTheDocument();
    });
  });

  describe('Language Display', () => {
    test('should show available languages in dropdown', async () => {
      await renderLanguageSelector();

      await user.click(screen.getByText('Select OCR languages...'));

      // Wait for languages to load
      await waitFor(() => {
        expect(screen.getByText('English')).toBeInTheDocument();
      });

      // Check for common languages
      expect(screen.getByText('Spanish')).toBeInTheDocument();
      expect(screen.getByText('French')).toBeInTheDocument();
      expect(screen.getByText('German')).toBeInTheDocument();
      expect(screen.getByText('Chinese (Simplified)')).toBeInTheDocument();
    });

    test('should handle less common languages', async () => {
      await renderLanguageSelector();

      await user.click(screen.getByText('Select OCR languages...'));

      // Wait for languages to load
      await waitFor(() => {
        expect(screen.getByText('Japanese')).toBeInTheDocument();
      });

      // Check for some less common languages
      expect(screen.getByText('Arabic')).toBeInTheDocument();
      expect(screen.getByText('Thai')).toBeInTheDocument();
    });
  });

  describe('Integration Scenarios', () => {
    test('should handle typical workflow: select language', async () => {
      const mockOnChange = vi.fn();
      await renderLanguageSelector({ onLanguagesChange: mockOnChange });

      // Start with no languages
      expect(screen.getByText('No languages selected. Documents will use default OCR language.')).toBeInTheDocument();

      // Open dropdown and wait for languages to load
      await user.click(screen.getByText('Select OCR languages...'));
      await waitFor(() => {
        expect(screen.getByText('English')).toBeInTheDocument();
      });

      // Select English
      await user.click(screen.getByText('English'));

      expect(mockOnChange).toHaveBeenCalledWith(['eng'], 'eng');
    });

    test('should handle selecting multiple languages', async () => {
      const mockOnChange = vi.fn();

      // Start with one language selected
      await renderLanguageSelector({
        selectedLanguages: ['eng'],
        primaryLanguage: 'eng',
        onLanguagesChange: mockOnChange
      });

      // Wait for API to load language names
      await waitFor(() => {
        expect(screen.getByText('English')).toBeInTheDocument();
      });
      expect(screen.getByText('(Primary)')).toBeInTheDocument();

      // Should show "Add more languages" button
      expect(screen.getByText('Add more languages (3 remaining)')).toBeInTheDocument();
    });

    test('should handle deselecting all languages', async () => {
      const mockOnChange = vi.fn();
      await renderLanguageSelector({
        selectedLanguages: [],
        primaryLanguage: '',
        onLanguagesChange: mockOnChange
      });

      expect(screen.getByText('No languages selected. Documents will use default OCR language.')).toBeInTheDocument();
    });
  });

  describe('Accessibility', () => {
    test('should be keyboard navigable', async () => {
      await renderLanguageSelector();

      const button = screen.getByText('Select OCR languages...').closest('button');

      // Tab to button and press Enter to open
      button?.focus();
      expect(button).toHaveFocus();

      await user.keyboard('{Enter}');

      // Wait for the dialog to appear
      await waitFor(() => {
        expect(screen.getByText('Available Languages')).toBeInTheDocument();
      });
    });

    test('should have proper button roles', async () => {
      await renderLanguageSelector();

      const button = screen.getByText('Select OCR languages...').closest('button');
      expect(button).toHaveAttribute('type', 'button');
    });

    test('should have proper structure when languages are selected', async () => {
      await renderLanguageSelector({
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng'
      });

      // Wait for API to load language names
      await waitFor(() => {
        expect(screen.getByText('English')).toBeInTheDocument();
      });
      expect(screen.getByText('Spanish')).toBeInTheDocument();

      // Should have proper button for adding more
      const addButton = screen.getByText('Add more languages (2 remaining)');
      expect(addButton.closest('button')).toHaveAttribute('type', 'button');
    });
  });

  describe('API Loading and Error States', () => {
    test('should show loading state while fetching languages', async () => {
      // Make the API call never resolve to keep loading state
      mockGetAvailableLanguages.mockImplementation(() => new Promise(() => {}));

      await renderLanguageSelector();
      await user.click(screen.getByText('Select OCR languages...'));

      expect(screen.getByText('Loading languages...')).toBeInTheDocument();
    });

    test('should show error state when API fails', async () => {
      mockGetAvailableLanguages.mockRejectedValue(new Error('Network error'));

      await renderLanguageSelector();
      await user.click(screen.getByText('Select OCR languages...'));

      await waitFor(() => {
        expect(screen.getByText('Failed to load languages')).toBeInTheDocument();
      });
    });

    test('should fallback to English when API fails', async () => {
      mockGetAvailableLanguages.mockRejectedValue(new Error('Network error'));

      await renderLanguageSelector();
      await user.click(screen.getByText('Select OCR languages...'));

      await waitFor(() => {
        expect(screen.getByText('English')).toBeInTheDocument();
      });
    });
  });
});
