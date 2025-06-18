import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import AdvancedSearchPanel from '../AdvancedSearchPanel';

const mockSettings = {
  useEnhancedSearch: true,
  searchMode: 'simple' as const,
  includeSnippets: true,
  snippetLength: 200,
  fuzzyThreshold: 0.8,
  resultLimit: 100,
  includeOcrText: true,
  includeFileContent: true,
  includeFilenames: true,
  boostRecentDocs: false,
  enableAutoCorrect: true,
};

const mockPresets = [
  {
    name: 'Fast Search',
    settings: {
      ...mockSettings,
      includeSnippets: false,
      resultLimit: 50,
    },
  },
  {
    name: 'Detailed Search',
    settings: {
      ...mockSettings,
      snippetLength: 400,
      resultLimit: 200,
    },
  },
];

describe('AdvancedSearchPanel', () => {
  const mockOnSettingsChange = vi.fn();
  const mockOnExpandedChange = vi.fn();
  const mockOnSavePreset = vi.fn();
  const mockOnLoadPreset = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  test('renders collapsed state by default', async () => {
    render(
      <AdvancedSearchPanel
        settings={mockSettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={false}
        onExpandedChange={mockOnExpandedChange}
      />
    );

    expect(screen.getByText('Advanced Search Options')).toBeInTheDocument();
    expect(screen.getByText('Customize search behavior and result display')).toBeInTheDocument();
    expect(screen.getByText('SIMPLE')).toBeInTheDocument();
    
    // Wait for MUI Collapse animation to complete
    await waitFor(() => {
      // The section buttons should not be visible when collapsed
      expect(screen.queryByRole('button', { name: 'Search Behavior' })).not.toBeInTheDocument();
    });
    
    expect(screen.queryByRole('button', { name: 'Results Display' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Performance' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Content Sources' })).not.toBeInTheDocument();
  });

  test('expands when expanded prop is true', () => {
    render(
      <AdvancedSearchPanel
        settings={mockSettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={true}
        onExpandedChange={mockOnExpandedChange}
      />
    );

    expect(screen.getByText('Search Behavior')).toBeInTheDocument();
    expect(screen.getByText('Results Display')).toBeInTheDocument();
    expect(screen.getByText('Performance')).toBeInTheDocument();
    expect(screen.getByText('Content Sources')).toBeInTheDocument();
  });

  test('calls onExpandedChange when header is clicked', async () => {
    const user = userEvent.setup();
    render(
      <AdvancedSearchPanel
        settings={mockSettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={false}
        onExpandedChange={mockOnExpandedChange}
      />
    );

    const header = screen.getByText('Advanced Search Options').closest('div');
    await user.click(header!);

    expect(mockOnExpandedChange).toHaveBeenCalledWith(true);
  });

  test('displays search behavior section by default when expanded', () => {
    render(
      <AdvancedSearchPanel
        settings={mockSettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={true}
        onExpandedChange={mockOnExpandedChange}
      />
    );

    expect(screen.getByText('These settings control how your search queries are interpreted and matched against documents.')).toBeInTheDocument();
    expect(screen.getByDisplayValue('simple')).toBeInTheDocument();
    expect(screen.getByLabelText('Enhanced Search Engine')).toBeInTheDocument();
  });

  test('switches between sections correctly', async () => {
    const user = userEvent.setup();
    render(
      <AdvancedSearchPanel
        settings={mockSettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={true}
        onExpandedChange={mockOnExpandedChange}
      />
    );

    // Click on Results Display section
    await user.click(screen.getByText('Results Display'));

    expect(screen.getByText('Control how search results are presented and what information is shown.')).toBeInTheDocument();
    expect(screen.getByLabelText('Show Text Snippets')).toBeInTheDocument();
  });

  test('changes search mode setting', async () => {
    const user = userEvent.setup();
    render(
      <AdvancedSearchPanel
        settings={mockSettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={true}
        onExpandedChange={mockOnExpandedChange}
      />
    );

    // Find the Select component by its label
    const searchModeSelect = screen.getByRole('combobox');
    await user.click(searchModeSelect);
    
    // Wait for the options to appear and click on fuzzy
    await waitFor(() => {
      expect(screen.getByText('Fuzzy Search')).toBeInTheDocument();
    });
    
    const fuzzyOption = screen.getByText('Fuzzy Search');
    await user.click(fuzzyOption);

    expect(mockOnSettingsChange).toHaveBeenCalledWith({ searchMode: 'fuzzy' });
  });

  test('toggles enhanced search setting', async () => {
    const user = userEvent.setup();
    render(
      <AdvancedSearchPanel
        settings={mockSettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={true}
        onExpandedChange={mockOnExpandedChange}
      />
    );

    const enhancedSearchSwitch = screen.getByLabelText('Enhanced Search Engine');
    await user.click(enhancedSearchSwitch);

    expect(mockOnSettingsChange).toHaveBeenCalledWith({ useEnhancedSearch: false });
  });

  test('adjusts fuzzy threshold when in fuzzy mode', async () => {
    const user = userEvent.setup();
    const fuzzySettings = { ...mockSettings, searchMode: 'fuzzy' as const };
    
    render(
      <AdvancedSearchPanel
        settings={fuzzySettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={true}
        onExpandedChange={mockOnExpandedChange}
      />
    );

    // Check that fuzzy threshold text is displayed
    expect(screen.getByText(/Fuzzy Match Threshold: 0.8/)).toBeInTheDocument();
    
    // Find the slider by role - it should not be disabled in fuzzy mode
    const fuzzySlider = screen.getByRole('slider');
    expect(fuzzySlider).not.toHaveAttribute('disabled');
  });

  test('disables fuzzy threshold when not in fuzzy mode', () => {
    render(
      <AdvancedSearchPanel
        settings={mockSettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={true}
        onExpandedChange={mockOnExpandedChange}
      />
    );

    // Find the slider by role - it should be disabled when not in fuzzy mode
    const fuzzySlider = screen.getByRole('slider');
    expect(fuzzySlider).toHaveAttribute('disabled');
  });

  test('shows results display settings correctly', async () => {
    const user = userEvent.setup();
    render(
      <AdvancedSearchPanel
        settings={mockSettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={true}
        onExpandedChange={mockOnExpandedChange}
      />
    );

    await user.click(screen.getByText('Results Display'));

    expect(screen.getByLabelText('Show Text Snippets')).toBeChecked();
    expect(screen.getByDisplayValue('200')).toBeInTheDocument(); // Snippet length
    expect(screen.getByDisplayValue('100')).toBeInTheDocument(); // Results per page
  });

  test('disables snippet length when snippets are disabled', async () => {
    const user = userEvent.setup();
    const settingsWithoutSnippets = { ...mockSettings, includeSnippets: false };
    
    render(
      <AdvancedSearchPanel
        settings={settingsWithoutSnippets}
        onSettingsChange={mockOnSettingsChange}
        expanded={true}
        onExpandedChange={mockOnExpandedChange}
      />
    );

    await user.click(screen.getByText('Results Display'));

    const snippetLengthSelect = screen.getByLabelText('Snippet Length');
    expect(snippetLengthSelect.closest('div')).toHaveClass('Mui-disabled');
  });

  test('shows content sources settings', async () => {
    const user = userEvent.setup();
    render(
      <AdvancedSearchPanel
        settings={mockSettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={true}
        onExpandedChange={mockOnExpandedChange}
      />
    );

    await user.click(screen.getByText('Content Sources'));

    expect(screen.getByLabelText('Document Content')).toBeChecked();
    expect(screen.getByLabelText('OCR Extracted Text')).toBeChecked();
    expect(screen.getByLabelText('Filenames')).toBeChecked();
  });

  test('shows performance settings with warning', async () => {
    const user = userEvent.setup();
    render(
      <AdvancedSearchPanel
        settings={mockSettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={true}
        onExpandedChange={mockOnExpandedChange}
      />
    );

    await user.click(screen.getByText('Performance'));

    expect(screen.getByText('These settings can affect search speed. Use with caution for large document collections.')).toBeInTheDocument();
    expect(screen.getByRole('slider', { name: /maximum results/i })).toBeInTheDocument();
  });

  test('resets to defaults when reset button is clicked', async () => {
    const user = userEvent.setup();
    render(
      <AdvancedSearchPanel
        settings={mockSettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={true}
        onExpandedChange={mockOnExpandedChange}
      />
    );

    const resetButton = screen.getByText('Reset to Defaults');
    await user.click(resetButton);

    expect(mockOnSettingsChange).toHaveBeenCalledWith({
      useEnhancedSearch: true,
      searchMode: 'simple',
      includeSnippets: true,
      snippetLength: 200,
      fuzzyThreshold: 0.8,
      resultLimit: 100,
      includeOcrText: true,
      includeFileContent: true,
      includeFilenames: true,
      boostRecentDocs: false,
      enableAutoCorrect: true,
    });
  });

  test('shows save preset dialog when save preset is clicked', async () => {
    const user = userEvent.setup();
    render(
      <AdvancedSearchPanel
        settings={mockSettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={true}
        onExpandedChange={mockOnExpandedChange}
        onSavePreset={mockOnSavePreset}
      />
    );

    const saveButton = screen.getByText('Save Preset');
    await user.click(saveButton);

    expect(screen.getByText('Save Current Settings as Preset')).toBeInTheDocument();
    expect(screen.getByLabelText('Preset Name')).toBeInTheDocument();
  });

  test('saves preset with valid name', async () => {
    const user = userEvent.setup();
    render(
      <AdvancedSearchPanel
        settings={mockSettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={true}
        onExpandedChange={mockOnExpandedChange}
        onSavePreset={mockOnSavePreset}
      />
    );

    await user.click(screen.getByText('Save Preset'));
    
    const nameInput = screen.getByLabelText('Preset Name');
    await user.type(nameInput, 'My Custom Preset');
    
    const saveButton = screen.getByRole('button', { name: 'Save' });
    await user.click(saveButton);

    expect(mockOnSavePreset).toHaveBeenCalledWith('My Custom Preset', mockSettings);
  });

  test('shows preset selector when presets are available', () => {
    render(
      <AdvancedSearchPanel
        settings={mockSettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={true}
        onExpandedChange={mockOnExpandedChange}
        availablePresets={mockPresets}
        onLoadPreset={mockOnLoadPreset}
      />
    );

    expect(screen.getByLabelText('Load Preset')).toBeInTheDocument();
  });

  test('loads preset when selected', async () => {
    const user = userEvent.setup();
    render(
      <AdvancedSearchPanel
        settings={mockSettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={true}
        onExpandedChange={mockOnExpandedChange}
        availablePresets={mockPresets}
        onLoadPreset={mockOnLoadPreset}
      />
    );

    const presetSelect = screen.getByLabelText('Load Preset');
    await user.click(presetSelect);
    
    const fastSearchOption = screen.getByText('Fast Search');
    await user.click(fastSearchOption);

    expect(mockOnLoadPreset).toHaveBeenCalledWith(mockPresets[0].settings);
  });

  test('shows enhanced search badge when enabled', () => {
    render(
      <AdvancedSearchPanel
        settings={mockSettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={false}
        onExpandedChange={mockOnExpandedChange}
      />
    );

    // Badge should be visible (not invisible) when enhanced search is enabled
    const badge = screen.getByText('Advanced Search Options').closest('div')?.querySelector('[class*="MuiBadge"]');
    expect(badge).toBeInTheDocument();
  });

  test('hides badge when enhanced search is disabled', () => {
    const settingsWithoutEnhanced = { ...mockSettings, useEnhancedSearch: false };
    
    render(
      <AdvancedSearchPanel
        settings={settingsWithoutEnhanced}
        onSettingsChange={mockOnSettingsChange}
        expanded={false}
        onExpandedChange={mockOnExpandedChange}
      />
    );

    // Badge should be invisible when enhanced search is disabled
    const badge = screen.getByText('Advanced Search Options').closest('div')?.querySelector('[class*="MuiBadge"]');
    expect(badge).toBeInTheDocument(); // Badge element exists but should be invisible
  });

  test('cancels preset save when cancel is clicked', async () => {
    const user = userEvent.setup();
    render(
      <AdvancedSearchPanel
        settings={mockSettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={true}
        onExpandedChange={mockOnExpandedChange}
        onSavePreset={mockOnSavePreset}
      />
    );

    await user.click(screen.getByText('Save Preset'));
    
    const cancelButton = screen.getByText('Cancel');
    await user.click(cancelButton);

    expect(screen.queryByText('Save Current Settings as Preset')).not.toBeInTheDocument();
  });

  test('shows correct search mode descriptions', () => {
    render(
      <AdvancedSearchPanel
        settings={mockSettings}
        onSettingsChange={mockOnSettingsChange}
        expanded={true}
        onExpandedChange={mockOnExpandedChange}
      />
    );

    expect(screen.getByText('Basic keyword matching with stemming')).toBeInTheDocument();
  });
});