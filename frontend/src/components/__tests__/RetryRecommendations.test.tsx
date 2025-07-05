import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { createComprehensiveAxiosMock, createComprehensiveApiMocks } from '../../test/comprehensive-mocks';

// Mock axios comprehensively to prevent any real HTTP requests
vi.mock('axios', () => createComprehensiveAxiosMock());

// Create mock functions for this specific test
const mockGetRetryRecommendations = vi.fn();
const mockBulkRetryOcr = vi.fn();

// Mock the API module with comprehensive mocking
vi.mock('../../services/api', async () => {
  const actual = await vi.importActual('../../services/api');
  const apiMocks = createComprehensiveApiMocks();
  
  return {
    ...actual,
    ...apiMocks,
    documentService: {
      ...apiMocks.documentService,
      getRetryRecommendations: mockGetRetryRecommendations,
      bulkRetryOcr: mockBulkRetryOcr,
    },
  };
});

// Import after mocking
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { RetryRecommendations } from '../RetryRecommendations';

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
  ];

  beforeEach(() => {
    vi.clearAllMocks();
    vi.resetAllMocks();
    
    // Reset mock props
    mockProps.onRetrySuccess.mockClear();
    mockProps.onRetryClick.mockClear();
    
    mockGetRetryRecommendations.mockResolvedValue({
      data: {
        recommendations: sampleRecommendations,
        total_recommendations: 1,
      },
    });
    mockBulkRetryOcr.mockResolvedValue({
      data: {
        success: true,
        queued_count: 10,
        matched_count: 15,
        documents: [],
        estimated_total_time_minutes: 5.2,
        message: 'Retry operation completed successfully',
      },
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
    vi.resetAllMocks();
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
      expect(screen.getByText(/No retry recommendations/)).toBeInTheDocument();
    });
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
      // Should not crash and show empty state
      expect(screen.getByText(/No retry recommendations/)).toBeInTheDocument();
    });
  });
});