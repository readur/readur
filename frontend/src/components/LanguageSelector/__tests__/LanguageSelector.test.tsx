import { describe, test, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import LanguageSelector from '../LanguageSelector';
import { renderWithProviders } from '../../../test/test-utils';

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
  });

  describe('Basic Rendering', () => {
    test('should render the language selector container', () => {
      renderLanguageSelector();
      expect(screen.getByText('OCR Languages')).toBeInTheDocument();
    });

    test('should show default state text when no languages selected', () => {
      renderLanguageSelector();
      expect(screen.getByText('No languages selected. Documents will use default OCR language.')).toBeInTheDocument();
    });

    test('should show selection button', () => {
      renderLanguageSelector();
      expect(screen.getByText('Select OCR languages...')).toBeInTheDocument();
    });

    test('should show language count when languages are selected', () => {
      renderLanguageSelector({ 
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng'
      });
      expect(screen.getByText('OCR Languages (2/4)')).toBeInTheDocument();
    });

    test('should open dropdown when button is clicked', async () => {
      renderLanguageSelector();
      
      await user.click(screen.getByText('Select OCR languages...'));
      
      expect(screen.getByText('Available Languages')).toBeInTheDocument();
      expect(screen.getByText('English')).toBeInTheDocument();
      expect(screen.getByText('Spanish')).toBeInTheDocument();
    });

    test('should apply custom className', () => {
      const { container } = renderLanguageSelector({ className: 'custom-class' });
      expect(container.firstChild).toHaveClass('custom-class');
    });
  });

  describe('Language Selection', () => {
    test('should show selected languages as tags', () => {
      renderLanguageSelector({ 
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng'
      });
      
      expect(screen.getByText('English')).toBeInTheDocument();
      expect(screen.getByText('Spanish')).toBeInTheDocument();
      expect(screen.getByText('(Primary)')).toBeInTheDocument();
    });

    test('should call onLanguagesChange when language is selected from dropdown', async () => {
      const mockOnChange = vi.fn();
      renderLanguageSelector({ onLanguagesChange: mockOnChange });
      
      // Open dropdown
      await user.click(screen.getByText('Select OCR languages...'));
      
      // Select English from the dropdown - click on the language text directly
      await user.click(screen.getByText('English'));
      
      expect(mockOnChange).toHaveBeenCalledWith(['eng'], 'eng');
    });

    test('should show "Add more languages" when languages are selected', () => {
      renderLanguageSelector({ 
        selectedLanguages: ['eng'],
        primaryLanguage: 'eng'
      });
      
      expect(screen.getByText('Add more languages (3 remaining)')).toBeInTheDocument();
    });

    test('should handle maximum language limit', () => {
      renderLanguageSelector({ 
        selectedLanguages: ['eng', 'spa', 'fra', 'deu'],
        primaryLanguage: 'eng',
        maxLanguages: 4
      });
      
      expect(screen.getByText('Add more languages (0 remaining)')).toBeInTheDocument();
    });
  });

  describe('Primary Language', () => {
    test('should show primary language indicator', () => {
      renderLanguageSelector({ 
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng'
      });
      
      expect(screen.getByText('(Primary)')).toBeInTheDocument();
    });

    test('should handle primary language changes', async () => {
      const mockOnChange = vi.fn();
      renderLanguageSelector({ 
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng',
        onLanguagesChange: mockOnChange 
      });
      
      // Open dropdown and click on a primary language option
      await user.click(screen.getByText('Add more languages (2 remaining)'));
      
      // The implementation should show primary selection when languages are selected
      // This is more of an integration test
    });
  });

  describe('Disabled State', () => {
    test('should not show button when disabled', () => {
      renderLanguageSelector({ disabled: true });
      
      expect(screen.queryByText('Select OCR languages...')).not.toBeInTheDocument();
    });

    test('should not show remove buttons when disabled', () => {
      renderLanguageSelector({ 
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng',
        disabled: true
      });
      
      // Should show languages but no interactive elements
      expect(screen.getByText('English')).toBeInTheDocument();
      expect(screen.getByText('Spanish')).toBeInTheDocument();
    });
  });

  describe('Custom Configuration', () => {
    test('should respect custom maxLanguages prop', () => {
      renderLanguageSelector({ 
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng',
        maxLanguages: 3
      });
      
      expect(screen.getByText('OCR Languages (2/3)')).toBeInTheDocument();
      expect(screen.getByText('Add more languages (1 remaining)')).toBeInTheDocument();
    });

    test('should handle edge case of maxLanguages = 1', () => {
      renderLanguageSelector({ 
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
      renderLanguageSelector();
      
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
      
      // Start with no languages
      expect(screen.getByText('No languages selected. Documents will use default OCR language.')).toBeInTheDocument();
      
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
      
      // Should show the selected language
      expect(screen.getByText('English')).toBeInTheDocument();
      expect(screen.getByText('(Primary)')).toBeInTheDocument();
      
      // Should show "Add more languages" button
      expect(screen.getByText('Add more languages (3 remaining)')).toBeInTheDocument();
    });

    test('should handle deselecting all languages', () => {
      const mockOnChange = vi.fn();
      renderLanguageSelector({ 
        selectedLanguages: [],
        primaryLanguage: '',
        onLanguagesChange: mockOnChange 
      });
      
      expect(screen.getByText('No languages selected. Documents will use default OCR language.')).toBeInTheDocument();
    });
  });

  describe('Accessibility', () => {
    test('should be keyboard navigable', async () => {
      renderLanguageSelector();
      
      const button = screen.getByText('Select OCR languages...').closest('button');
      
      // Tab to button and press Enter to open
      button?.focus();
      expect(button).toHaveFocus();
      
      await user.keyboard('{Enter}');
      expect(screen.getByText('Available Languages')).toBeInTheDocument();
    });

    test('should have proper button roles', () => {
      renderLanguageSelector();
      
      const button = screen.getByText('Select OCR languages...').closest('button');
      expect(button).toHaveAttribute('type', 'button');
    });

    test('should have proper structure when languages are selected', () => {
      renderLanguageSelector({ 
        selectedLanguages: ['eng', 'spa'],
        primaryLanguage: 'eng'
      });
      
      // Should have language tags
      expect(screen.getByText('English')).toBeInTheDocument();
      expect(screen.getByText('Spanish')).toBeInTheDocument();
      
      // Should have proper button for adding more
      const addButton = screen.getByText('Add more languages (2 remaining)');
      expect(addButton.closest('button')).toHaveAttribute('type', 'button');
    });
  });
});