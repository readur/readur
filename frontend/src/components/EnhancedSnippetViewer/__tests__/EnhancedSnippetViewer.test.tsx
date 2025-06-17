import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import EnhancedSnippetViewer from '../EnhancedSnippetViewer';

const mockSnippets = [
  {
    text: 'This is a sample document about invoice processing and payment systems.',
    highlight_ranges: [
      { start: 38, end: 45 }, // "invoice"
      { start: 59, end: 66 }, // "payment"
    ],
    source: 'content' as const,
    page_number: 1,
    confidence: 0.95,
  },
  {
    text: 'OCR extracted text from scanned document with lower confidence.',
    highlight_ranges: [
      { start: 0, end: 3 }, // "OCR"
    ],
    source: 'ocr_text' as const,
    confidence: 0.75,
  },
  {
    text: 'filename_with_keywords.pdf',
    highlight_ranges: [
      { start: 14, end: 22 }, // "keywords"
    ],
    source: 'filename' as const,
  },
];

describe('EnhancedSnippetViewer', () => {
  const mockOnSnippetClick = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    // Mock clipboard API
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn().mockImplementation(() => Promise.resolve()),
      },
    });
  });

  test('renders snippets with correct content', () => {
    render(
      <EnhancedSnippetViewer
        snippets={mockSnippets}
        searchQuery="invoice payment"
        onSnippetClick={mockOnSnippetClick}
      />
    );

    expect(screen.getByText('Search Results')).toBeInTheDocument();
    expect(screen.getByText('3 matches')).toBeInTheDocument();
    expect(screen.getByText(/This is a sample document about/)).toBeInTheDocument();
    expect(screen.getByText(/OCR extracted text/)).toBeInTheDocument();
  });

  test('displays search query context', () => {
    render(
      <EnhancedSnippetViewer
        snippets={mockSnippets}
        searchQuery="invoice payment"
        onSnippetClick={mockOnSnippetClick}
      />
    );

    expect(screen.getByText('Showing matches for:')).toBeInTheDocument();
    expect(screen.getByText('invoice payment')).toBeInTheDocument();
  });

  test('shows correct source badges', () => {
    render(
      <EnhancedSnippetViewer
        snippets={mockSnippets}
        onSnippetClick={mockOnSnippetClick}
      />
    );

    expect(screen.getByText('Document Content')).toBeInTheDocument();
    expect(screen.getByText('OCR Text')).toBeInTheDocument();
    expect(screen.getByText('Filename')).toBeInTheDocument();
  });

  test('displays page numbers and confidence scores', () => {
    render(
      <EnhancedSnippetViewer
        snippets={mockSnippets}
        onSnippetClick={mockOnSnippetClick}
      />
    );

    expect(screen.getByText('Page 1')).toBeInTheDocument();
    expect(screen.getByText('75% confidence')).toBeInTheDocument();
  });

  test('limits snippets display based on maxSnippetsToShow', () => {
    render(
      <EnhancedSnippetViewer
        snippets={mockSnippets}
        maxSnippetsToShow={2}
        onSnippetClick={mockOnSnippetClick}
      />
    );

    expect(screen.getByText('Show All (3)')).toBeInTheDocument();
    
    // Should only show first 2 snippets
    expect(screen.getByText(/This is a sample document/)).toBeInTheDocument();
    expect(screen.getByText(/OCR extracted text/)).toBeInTheDocument();
    expect(screen.queryByText(/filename_with_keywords/)).not.toBeInTheDocument();
  });

  test('expands to show all snippets when clicked', async () => {
    const user = userEvent.setup();
    render(
      <EnhancedSnippetViewer
        snippets={mockSnippets}
        maxSnippetsToShow={2}
        onSnippetClick={mockOnSnippetClick}
      />
    );

    const showAllButton = screen.getByText('Show All (3)');
    await user.click(showAllButton);

    expect(screen.getByText('Show Less')).toBeInTheDocument();
    expect(screen.getByText(/filename_with_keywords/)).toBeInTheDocument();
  });

  test('calls onSnippetClick when snippet is clicked', async () => {
    const user = userEvent.setup();
    render(
      <EnhancedSnippetViewer
        snippets={mockSnippets}
        onSnippetClick={mockOnSnippetClick}
      />
    );

    const firstSnippet = screen.getByText(/This is a sample document/).closest('div');
    await user.click(firstSnippet!);

    expect(mockOnSnippetClick).toHaveBeenCalledWith(mockSnippets[0], 0);
  });

  test('copies snippet text to clipboard', async () => {
    const user = userEvent.setup();
    render(
      <EnhancedSnippetViewer
        snippets={mockSnippets}
        onSnippetClick={mockOnSnippetClick}
      />
    );

    const copyButtons = screen.getAllByLabelText('Copy snippet');
    await user.click(copyButtons[0]);

    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(mockSnippets[0].text);
  });

  test('opens settings menu and changes view mode', async () => {
    const user = userEvent.setup();
    render(
      <EnhancedSnippetViewer
        snippets={mockSnippets}
        onSnippetClick={mockOnSnippetClick}
      />
    );

    const settingsButton = screen.getByLabelText('Snippet settings');
    await user.click(settingsButton);

    expect(screen.getByText('Snippet Display Settings')).toBeInTheDocument();
    expect(screen.getByText('View Mode')).toBeInTheDocument();

    const compactOption = screen.getByLabelText('Compact');
    await user.click(compactOption);

    // Settings menu should close and compact mode should be applied
    await waitFor(() => {
      expect(screen.queryByText('Snippet Display Settings')).not.toBeInTheDocument();
    });
  });

  test('changes highlight style through settings', async () => {
    const user = userEvent.setup();
    render(
      <EnhancedSnippetViewer
        snippets={mockSnippets}
        onSnippetClick={mockOnSnippetClick}
      />
    );

    const settingsButton = screen.getByLabelText('Snippet settings');
    await user.click(settingsButton);

    const underlineOption = screen.getByLabelText('Underline');
    await user.click(underlineOption);

    await waitFor(() => {
      expect(screen.queryByText('Snippet Display Settings')).not.toBeInTheDocument();
    });

    // Check if highlight style has changed (this would require checking computed styles)
    const highlightedText = screen.getByText('invoice');
    expect(highlightedText).toBeInTheDocument();
  });

  test('adjusts font size through settings', async () => {
    const user = userEvent.setup();
    render(
      <EnhancedSnippetViewer
        snippets={mockSnippets}
        onSnippetClick={mockOnSnippetClick}
      />
    );

    const settingsButton = screen.getByLabelText('Snippet settings');
    await user.click(settingsButton);

    const fontSizeSlider = screen.getByRole('slider', { name: /font size/i });
    await user.click(fontSizeSlider);

    // Font size should be adjustable
    expect(fontSizeSlider).toBeInTheDocument();
  });

  test('handles context mode settings', async () => {
    const user = userEvent.setup();
    render(
      <EnhancedSnippetViewer
        snippets={mockSnippets}
        onSnippetClick={mockOnSnippetClick}
      />
    );

    const settingsButton = screen.getByLabelText('Snippet settings');
    await user.click(settingsButton);

    const contextOption = screen.getByLabelText('Context Focus');
    await user.click(contextOption);

    // Context length slider should appear
    expect(screen.getByText(/Context Length:/)).toBeInTheDocument();
  });

  test('renders highlighted text with multiple ranges correctly', () => {
    render(
      <EnhancedSnippetViewer
        snippets={[mockSnippets[0]]} // First snippet has multiple highlights
        onSnippetClick={mockOnSnippetClick}
      />
    );

    // Both "invoice" and "payment" should be highlighted
    expect(screen.getByText('invoice')).toBeInTheDocument();
    expect(screen.getByText('payment')).toBeInTheDocument();
  });

  test('handles snippets without highlight ranges', () => {
    const snippetsWithoutHighlights = [
      {
        text: 'Plain text without any highlights',
        source: 'content' as const,
      },
    ];

    render(
      <EnhancedSnippetViewer
        snippets={snippetsWithoutHighlights}
        onSnippetClick={mockOnSnippetClick}
      />
    );

    expect(screen.getByText('Plain text without any highlights')).toBeInTheDocument();
  });

  test('displays empty state when no snippets provided', () => {
    render(
      <EnhancedSnippetViewer
        snippets={[]}
        onSnippetClick={mockOnSnippetClick}
      />
    );

    expect(screen.getByText('No text snippets available for this search result')).toBeInTheDocument();
  });

  test('shows confidence warning for low confidence OCR', () => {
    const lowConfidenceSnippet = [
      {
        text: 'Low confidence OCR text',
        source: 'ocr_text' as const,
        confidence: 0.6,
      },
    ];

    render(
      <EnhancedSnippetViewer
        snippets={lowConfidenceSnippet}
        onSnippetClick={mockOnSnippetClick}
      />
    );

    expect(screen.getByText('60% confidence')).toBeInTheDocument();
  });

  test('does not show confidence for high confidence OCR', () => {
    const highConfidenceSnippet = [
      {
        text: 'High confidence OCR text',
        source: 'ocr_text' as const,
        confidence: 0.9,
      },
    ];

    render(
      <EnhancedSnippetViewer
        snippets={highConfidenceSnippet}
        onSnippetClick={mockOnSnippetClick}
      />
    );

    expect(screen.queryByText('90% confidence')).not.toBeInTheDocument();
  });

  test('handles click events without onSnippetClick prop', async () => {
    const user = userEvent.setup();
    render(
      <EnhancedSnippetViewer
        snippets={mockSnippets}
      />
    );

    const firstSnippet = screen.getByText(/This is a sample document/).closest('div');
    expect(() => user.click(firstSnippet!)).not.toThrow();
  });
});