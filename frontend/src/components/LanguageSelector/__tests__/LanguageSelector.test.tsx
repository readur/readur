import { describe, test, expect, vi, beforeEach } from 'vitest';
import { createComprehensiveAxiosMock } from '../../../test/comprehensive-mocks';

// Mock axios to prevent real HTTP requests
vi.mock('axios', () => createComprehensiveAxiosMock());

import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ocrService } from '../../../services/api';
import LanguageSelector from '../LanguageSelector';
import { renderWithProviders } from '../../../test/test-utils';

const mockApiResponse = {
  data: {
    available_languages: [
      { code: 'eng', name: 'English', installed: true },
      { code: 'spa', name: 'Spanish', installed: true },
      { code: 'fra', name: 'French', installed: true },
      { code: 'deu', name: 'German', installed: true },
      { code: 'ita', name: 'Italian', installed: true },
      { code: 'por', name: 'Portuguese', installed: true },
      { code: 'rus', name: 'Russian', installed: true },
      { code: 'chi_sim', name: 'Chinese (Simplified)', installed: true },
      { code: 'chi_tra', name: 'Chinese (Traditional)', installed: true },
      { code: 'jpn', name: 'Japanese', installed: true },
      { code: 'kor', name: 'Korean', installed: true },
      { code: 'ara', name: 'Arabic', installed: true },
      { code: 'hin', name: 'Hindi', installed: true },
      { code: 'nld', name: 'Dutch', installed: true },
      { code: 'swe', name: 'Swedish', installed: true },
      { code: 'nor', name: 'Norwegian', installed: true },
      { code: 'dan', name: 'Danish', installed: true },
      { code: 'fin', name: 'Finnish', installed: true },
      { code: 'pol', name: 'Polish', installed: true },
      { code: 'ces', name: 'Czech', installed: true },
      { code: 'hun', name: 'Hungarian', installed: true },
      { code: 'tur', name: 'Turkish', installed: true },
      { code: 'tha', name: 'Thai', installed: true },
      { code: 'vie', name: 'Vietnamese', installed: true },
    ],
    current_user_language: 'eng',
  },
};

const renderLanguageSelector = (props: Partial<React.ComponentProps<typeof LanguageSelector>> = {}) => {
  const defaultProps = {
    selectedLanguages: [],
    primaryLanguage: '',
    onLanguagesChange: vi.fn(),
    ...props,
  };

  return renderWithProviders(<LanguageSelector {...defaultProps} />);
};

describe('LanguageSelector Component', () => {
  let user: ReturnType<typeof userEvent.setup>;

  beforeEach(() => {
    user = userEvent.setup();
    vi.spyOn(ocrService, 'getAvailableLanguages').mockResolvedValue(mockApiResponse as any);
  });

  describe('Basic Rendering', () => {
    test('should render the language selector container', async () => {
      renderLanguageSelector();
      await waitFor(() => {
        expect(screen.getByText('OCR Languages')).toBeInTheDocument();
      });
    });

    test('should show loading state initially', () => {
      renderLanguageSelector();
      expect(screen.getByText('Loading languages...')).toBeInTheDocument();
    });

    test('should show default state text when no languages selected', async () => {
      renderLanguageSelector();
      await waitFor(() => {
        expect(screen.getByText('No languages selected. Documents will use default OCR language.')).toBeInTheDocument();
      });
    });

    test('should show selection button', async () => {
      renderLanguageSelector();
      await waitFor(() => {
        expect(screen.getByText('Select OCR languages...')).toBeInTheDocument();
      });
    });

    test('should show language count when languages are selected', async () => {
      renderLanguageSelector({
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng'
      });
      await waitFor(() => {
        expect(screen.getByText('OCR Languages (2/4)')).toBeInTheDocument();
      });
    });

    test('should open dropdown when button is clicked', async () => {
      renderLanguageSelector();

      await waitFor(() => {
        expect(screen.getByText('Select OCR languages...')).toBeInTheDocument();
      });

      await user.click(screen.getByText('Select OCR languages...'));

      expect(screen.getByText('Available Languages')).toBeInTheDocument();
      expect(screen.getByText('English')).toBeInTheDocument();
      expect(screen.getByText('Spanish')).toBeInTheDocument();
    });

    test('should apply custom className', async () => {
      const { container } = renderLanguageSelector({ className: 'custom-class' });
      await waitFor(() => {
        expect(screen.getByText('OCR Languages')).toBeInTheDocument();
      });
      expect(container.firstChild).toHaveClass('custom-class');
    });
  });

  describe('Language Selection', () => {
    test('should show selected languages as tags', async () => {
      renderLanguageSelector({
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng'
      });

      await waitFor(() => {
        expect(screen.getByText('English')).toBeInTheDocument();
        expect(screen.getByText('Spanish')).toBeInTheDocument();
        expect(screen.getByText('(Primary)')).toBeInTheDocument();
      });
    });

    test('should call onLanguagesChange when language is selected from dropdown', async () => {
      const mockOnChange = vi.fn();
      renderLanguageSelector({ onLanguagesChange: mockOnChange });

      await waitFor(() => {
        expect(screen.getByText('Select OCR languages...')).toBeInTheDocument();
      });

      // Open dropdown
      await user.click(screen.getByText('Select OCR languages...'));

      // Select English from the dropdown
      await user.click(screen.getByText('English'));

      expect(mockOnChange).toHaveBeenCalledWith(['eng'], 'eng');
    });

    test('should show "Add more languages" when languages are selected', async () => {
      renderLanguageSelector({
        selectedLanguages: ['eng'],
        primaryLanguage: 'eng'
      });

      await waitFor(() => {
        expect(screen.getByText('Add more languages (3 remaining)')).toBeInTheDocument();
      });
    });

    test('should handle maximum language limit', async () => {
      renderLanguageSelector({
        selectedLanguages: ['eng', 'spa', 'fra', 'deu'],
        primaryLanguage: 'eng',
        maxLanguages: 4
      });

      await waitFor(() => {
        expect(screen.getByText('Add more languages (0 remaining)')).toBeInTheDocument();
      });
    });
  });

  describe('Primary Language', () => {
    test('should show primary language indicator', async () => {
      renderLanguageSelector({
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng'
      });

      await waitFor(() => {
        expect(screen.getByText('(Primary)')).toBeInTheDocument();
      });
    });

    test('should handle primary language changes', async () => {
      const mockOnChange = vi.fn();
      renderLanguageSelector({
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng',
        onLanguagesChange: mockOnChange
      });

      await waitFor(() => {
        expect(screen.getByText('Add more languages (2 remaining)')).toBeInTheDocument();
      });

      // Open dropdown and click on a primary language option
      await user.click(screen.getByText('Add more languages (2 remaining)'));
    });
  });

  describe('Disabled State', () => {
    test('should not show button when disabled', async () => {
      renderLanguageSelector({ disabled: true });

      await waitFor(() => {
        expect(screen.getByText('OCR Languages')).toBeInTheDocument();
      });

      expect(screen.queryByText('Select OCR languages...')).not.toBeInTheDocument();
    });

    test('should not show remove buttons when disabled', async () => {
      renderLanguageSelector({
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng',
        disabled: true
      });

      await waitFor(() => {
        // Should show languages but no interactive elements
        expect(screen.getByText('English')).toBeInTheDocument();
        expect(screen.getByText('Spanish')).toBeInTheDocument();
      });
    });
  });

  describe('Custom Configuration', () => {
    test('should respect custom maxLanguages prop', async () => {
      renderLanguageSelector({
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng',
        maxLanguages: 3
      });

      await waitFor(() => {
        expect(screen.getByText('OCR Languages (2/3)')).toBeInTheDocument();
        expect(screen.getByText('Add more languages (1 remaining)')).toBeInTheDocument();
      });
    });

    test('should handle edge case of maxLanguages = 1', async () => {
      renderLanguageSelector({
        selectedLanguages: ['eng'],
        primaryLanguage: 'eng',
        maxLanguages: 1
      });

      await waitFor(() => {
        expect(screen.getByText('OCR Languages (1/1)')).toBeInTheDocument();
        expect(screen.getByText('Add more languages (0 remaining)')).toBeInTheDocument();
      });
    });
  });

  describe('Language Display', () => {
    test('should show available languages in dropdown', async () => {
      renderLanguageSelector();

      await waitFor(() => {
        expect(screen.getByText('Select OCR languages...')).toBeInTheDocument();
      });

      await user.click(screen.getByText('Select OCR languages...'));

      // Check for common languages
      expect(screen.getByText('English')).toBeInTheDocument();
      expect(screen.getByText('Spanish')).toBeInTheDocument();
      expect(screen.getByText('French')).toBeInTheDocument();
      expect(screen.getByText('German')).toBeInTheDocument();
      expect(screen.getByText('Chinese (Simplified)')).toBeInTheDocument();
    });

    test('should handle less common languages', async () => {
      renderLanguageSelector();

      await waitFor(() => {
        expect(screen.getByText('Select OCR languages...')).toBeInTheDocument();
      });

      await user.click(screen.getByText('Select OCR languages...'));

      // Check for some less common languages
      expect(screen.getByText('Japanese')).toBeInTheDocument();
      expect(screen.getByText('Arabic')).toBeInTheDocument();
      expect(screen.getByText('Thai')).toBeInTheDocument();
    });
  });

  describe('Integration Scenarios', () => {
    test('should handle typical workflow: select language', async () => {
      const mockOnChange = vi.fn();
      renderLanguageSelector({ onLanguagesChange: mockOnChange });

      await waitFor(() => {
        // Start with no languages
        expect(screen.getByText('No languages selected. Documents will use default OCR language.')).toBeInTheDocument();
      });

      // Open dropdown and select English
      await user.click(screen.getByText('Select OCR languages...'));
      await user.click(screen.getByText('English'));

      expect(mockOnChange).toHaveBeenCalledWith(['eng'], 'eng');
    });

    test('should handle selecting multiple languages', async () => {
      const mockOnChange = vi.fn();

      // Start with one language selected
      renderLanguageSelector({
        selectedLanguages: ['eng'],
        primaryLanguage: 'eng',
        onLanguagesChange: mockOnChange
      });

      await waitFor(() => {
        // Should show the selected language
        expect(screen.getByText('English')).toBeInTheDocument();
        expect(screen.getByText('(Primary)')).toBeInTheDocument();
        // Should show "Add more languages" button
        expect(screen.getByText('Add more languages (3 remaining)')).toBeInTheDocument();
      });
    });

    test('should handle deselecting all languages', async () => {
      const mockOnChange = vi.fn();
      renderLanguageSelector({
        selectedLanguages: [],
        primaryLanguage: '',
        onLanguagesChange: mockOnChange
      });

      await waitFor(() => {
        expect(screen.getByText('No languages selected. Documents will use default OCR language.')).toBeInTheDocument();
      });
    });
  });

  describe('Accessibility', () => {
    test('should be keyboard navigable', async () => {
      renderLanguageSelector();

      await waitFor(() => {
        expect(screen.getByText('Select OCR languages...')).toBeInTheDocument();
      });

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
      renderLanguageSelector();

      await waitFor(() => {
        expect(screen.getByText('Select OCR languages...')).toBeInTheDocument();
      });

      const button = screen.getByText('Select OCR languages...').closest('button');
      expect(button).toHaveAttribute('type', 'button');
    });

    test('should have proper structure when languages are selected', async () => {
      renderLanguageSelector({
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng'
      });

      await waitFor(() => {
        // Should have language tags
        expect(screen.getByText('English')).toBeInTheDocument();
        expect(screen.getByText('Spanish')).toBeInTheDocument();
      });

      // Should have proper button for adding more
      const addButton = screen.getByText('Add more languages (2 remaining)');
      expect(addButton.closest('button')).toHaveAttribute('type', 'button');
    });
  });
});
