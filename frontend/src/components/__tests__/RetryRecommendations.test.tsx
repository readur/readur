import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { RetryRecommendations } from '../RetryRecommendations';

// Mock the API
const mockGetRetryRecommendations = vi.fn();
const mockBulkRetryOcr = vi.fn();

const mockDocumentService = {
  getRetryRecommendations: mockGetRetryRecommendations,
};

const mockApi = {
  bulkRetryOcr: mockBulkRetryOcr,
};

vi.mock('../../services/api', () => ({
  documentService: mockDocumentService,
  default: mockApi,
}));

describe('RetryRecommendations', () => {
  const mockProps = {
    onRetrySuccess: vi.fn(),
    onRetryClick: vi.fn(),
  };

  const sampleRecommendations = [
    {
      reason: 'low_confidence',
      title: 'Low Confidence Results',
      description: 'Documents with OCR confidence below 70%',
      estimated_success_rate: 0.8,
      document_count: 15,
      filter: {
        failure_reasons: ['low_confidence'],
        min_confidence: 0,
        max_confidence: 70,
      },
    },
    {
      reason: 'image_quality',
      title: 'Image Quality Issues',
      description: 'Documents that failed due to poor image quality',
      estimated_success_rate: 0.6,
      document_count: 8,
      filter: {
        failure_reasons: ['image_quality', 'resolution_too_low'],
      },
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
    mockGetRetryRecommendations.mockResolvedValue({
      data: {
        recommendations: sampleRecommendations,
        total_recommendations: 2,
      },
    });
    mockBulkRetryOcr.mockResolvedValue({
      data: {
        success: true,
        queued_count: 10,
        matched_count: 15,
        documents: [],
      },
    });
  });

  test('renders loading state initially', () => {
    mockGetRetryRecommendations.mockImplementation(() => new Promise(() => {})); // Never resolves
    render(<RetryRecommendations {...mockProps} />);

    expect(screen.getByRole('progressbar')).toBeInTheDocument();
    expect(screen.getByText('Loading retry recommendations...')).toBeInTheDocument();
  });

  test('loads and displays recommendations on mount', async () => {
    render(<RetryRecommendations {...mockProps} />);

    await waitFor(() => {
      expect(screen.getByText('OCR Retry Recommendations')).toBeInTheDocument();
    });

    expect(screen.getByText('Low Confidence Results')).toBeInTheDocument();
    expect(screen.getByText('Image Quality Issues')).toBeInTheDocument();
    expect(screen.getByText('15 documents')).toBeInTheDocument();
    expect(screen.getByText('8 documents')).toBeInTheDocument();
  });

  test('displays success rate badges with correct colors', async () => {
    render(<RetryRecommendations {...mockProps} />);

    await waitFor(() => {
      expect(screen.getByText('80% (High)')).toBeInTheDocument();
      expect(screen.getByText('60% (Medium)')).toBeInTheDocument();
    });

    // Check that the badges have the correct colors
    const highBadge = screen.getByText('80% (High)').closest('.MuiChip-root');
    const mediumBadge = screen.getByText('60% (Medium)').closest('.MuiChip-root');

    expect(highBadge).toHaveClass('MuiChip-colorSuccess');
    expect(mediumBadge).toHaveClass('MuiChip-colorWarning');
  });

  test('handles retry click with onRetryClick callback', async () => {
    const user = userEvent.setup();
    render(<RetryRecommendations {...mockProps} />);

    await waitFor(() => {
      expect(screen.getByText('Low Confidence Results')).toBeInTheDocument();
    });

    const retryButton = screen.getAllByText('Retry Now')[0];
    await user.click(retryButton);

    expect(mockProps.onRetryClick).toHaveBeenCalledWith(sampleRecommendations[0]);
  });

  test('executes retry directly when onRetryClick is not provided', async () => {
    const user = userEvent.setup();
    render(<RetryRecommendations onRetrySuccess={mockProps.onRetrySuccess} />);

    await waitFor(() => {
      expect(screen.getByText('Low Confidence Results')).toBeInTheDocument();
    });

    const retryButton = screen.getAllByText('Retry Now')[0];
    await user.click(retryButton);

    await waitFor(() => {
      expect(mockBulkRetryOcr).toHaveBeenCalledWith({
        mode: 'filter',
        filter: sampleRecommendations[0].filter,
        priority_override: 12,
      });
    });

    expect(mockProps.onRetrySuccess).toHaveBeenCalled();
  });

  test('shows loading state during retry execution', async () => {
    const user = userEvent.setup();
    mockBulkRetryOcr.mockImplementation(() => new Promise(resolve => 
      setTimeout(() => resolve({
        data: { success: true, queued_count: 10, matched_count: 10, documents: [] }
      }), 100)
    ));

    render(<RetryRecommendations onRetrySuccess={mockProps.onRetrySuccess} />);

    await waitFor(() => {
      expect(screen.getByText('Low Confidence Results')).toBeInTheDocument();
    });

    const retryButton = screen.getAllByText('Retry Now')[0];
    await user.click(retryButton);

    // Should show loading state
    expect(screen.getByRole('progressbar')).toBeInTheDocument();
    expect(retryButton).toBeDisabled();
  });

  test('handles API errors gracefully', async () => {
    mockGetRetryRecommendations.mockRejectedValue(new Error('API Error'));
    render(<RetryRecommendations {...mockProps} />);

    await waitFor(() => {
      expect(screen.getByText(/Failed to load retry recommendations/)).toBeInTheDocument();
    });
  });

  test('handles retry API errors gracefully', async () => {
    const user = userEvent.setup();
    mockBulkRetryOcr.mockRejectedValue({ 
      response: { data: { message: 'Retry failed' } } 
    });

    render(<RetryRecommendations onRetrySuccess={mockProps.onRetrySuccess} />);

    await waitFor(() => {
      expect(screen.getByText('Low Confidence Results')).toBeInTheDocument();
    });

    const retryButton = screen.getAllByText('Retry Now')[0];
    await user.click(retryButton);

    await waitFor(() => {
      expect(screen.getByText('Retry failed')).toBeInTheDocument();
    });
  });

  test('shows empty state when no recommendations are available', async () => {
    mockGetRetryRecommendations.mockResolvedValue({
      data: {
        recommendations: [],
        total_recommendations: 0,
      },
    });

    render(<RetryRecommendations {...mockProps} />);

    await waitFor(() => {
      expect(screen.getByText('No retry recommendations available')).toBeInTheDocument();
    });

    expect(screen.getByText('All documents have been processed successfully')).toBeInTheDocument();
    expect(screen.getByText('No failed documents found')).toBeInTheDocument();
  });

  test('shows correct success rate labels', () => {
    const { rerender } = render(<div />);

    // Test high success rate (>= 70%)
    mockGetRetryRecommendations.mockResolvedValue({
      data: {
        recommendations: [{
          ...sampleRecommendations[0],
          estimated_success_rate: 0.85,
        }],
        total_recommendations: 1,
      },
    });

    rerender(<RetryRecommendations {...mockProps} />);

    waitFor(() => {
      expect(screen.getByText('85% (High)')).toBeInTheDocument();
    });

    // Test medium success rate (40-69%)
    mockGetRetryRecommendations.mockResolvedValue({
      data: {
        recommendations: [{
          ...sampleRecommendations[0],
          estimated_success_rate: 0.55,
        }],
        total_recommendations: 1,
      },
    });

    rerender(<RetryRecommendations {...mockProps} />);

    waitFor(() => {
      expect(screen.getByText('55% (Medium)')).toBeInTheDocument();
    });

    // Test low success rate (< 40%)
    mockGetRetryRecommendations.mockResolvedValue({
      data: {
        recommendations: [{
          ...sampleRecommendations[0],
          estimated_success_rate: 0.25,
        }],
        total_recommendations: 1,
      },
    });

    rerender(<RetryRecommendations {...mockProps} />);

    waitFor(() => {
      expect(screen.getByText('25% (Low)')).toBeInTheDocument();
    });
  });

  test('refreshes recommendations after successful retry', async () => {
    const user = userEvent.setup();
    render(<RetryRecommendations onRetrySuccess={mockProps.onRetrySuccess} />);

    await waitFor(() => {
      expect(screen.getByText('Low Confidence Results')).toBeInTheDocument();
    });

    expect(mockGetRetryRecommendations).toHaveBeenCalledTimes(1);

    const retryButton = screen.getAllByText('Retry Now')[0];
    await user.click(retryButton);

    await waitFor(() => {
      expect(mockBulkRetryOcr).toHaveBeenCalled();
    });

    // Should reload recommendations after successful retry
    expect(mockGetRetryRecommendations).toHaveBeenCalledTimes(2);
  });

  test('handles null/undefined recommendations safely', async () => {
    mockGetRetryRecommendations.mockResolvedValue({
      data: {
        recommendations: null,
        total_recommendations: 0,
      },
    });

    render(<RetryRecommendations {...mockProps} />);

    await waitFor(() => {
      expect(screen.getByText('No retry recommendations available')).toBeInTheDocument();
    });

    // Should not crash
    expect(screen.getByText('OCR Retry Recommendations')).toBeInTheDocument();
  });
});