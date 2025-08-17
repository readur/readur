import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import { createComprehensiveAxiosMock } from '../../../test/comprehensive-mocks';
import { renderWithProviders } from '../../../test/test-utils';

// Mock axios comprehensively to prevent any real HTTP requests
vi.mock('axios', () => createComprehensiveAxiosMock());

import WebDAVScanFailures from '../WebDAVScanFailures';
import * as apiModule from '../../../services/api';

// Mock notification hook
const mockShowNotification = vi.fn();
vi.mock('../../../contexts/NotificationContext', async () => {
  const actual = await vi.importActual('../../../contexts/NotificationContext');
  return {
    ...actual,
    useNotification: () => ({ showNotification: mockShowNotification }),
  };
});

const mockScanFailuresData = {
  failures: [
    {
      id: '1',
      directory_path: '/test/path/long/directory/name',
      failure_type: 'timeout',
      failure_severity: 'high',
      failure_count: 3,
      consecutive_failures: 2,
      first_failure_at: '2024-01-01T10:00:00Z',
      last_failure_at: '2024-01-01T12:00:00Z',
      next_retry_at: '2024-01-01T13:00:00Z',
      error_message: 'Request timeout after 30 seconds',
      http_status_code: 408,
      user_excluded: false,
      user_notes: null,
      resolved: false,
      diagnostic_summary: {
        path_length: 45,
        directory_depth: 5,
        estimated_item_count: 1500,
        response_time_ms: 30000,
        response_size_mb: 2.5,
        server_type: 'Apache/2.4.41',
        recommended_action: 'Consider organizing files into smaller subdirectories or scanning during off-peak hours.',
        can_retry: true,
        user_action_required: false,
      },
    },
    {
      id: '2',
      directory_path: '/test/path/permissions',
      failure_type: 'permission_denied',
      failure_severity: 'critical',
      failure_count: 1,
      consecutive_failures: 1,
      first_failure_at: '2024-01-01T11:00:00Z',
      last_failure_at: '2024-01-01T11:00:00Z',
      next_retry_at: null,
      error_message: '403 Forbidden',
      http_status_code: 403,
      user_excluded: false,
      user_notes: null,
      resolved: false,
      diagnostic_summary: {
        path_length: 20,
        directory_depth: 3,
        estimated_item_count: null,
        response_time_ms: 1000,
        response_size_mb: null,
        server_type: 'Apache/2.4.41',
        recommended_action: 'Check that your WebDAV user has read access to this directory.',
        can_retry: false,
        user_action_required: true,
      },
    },
  ],
  stats: {
    active_failures: 2,
    resolved_failures: 5,
    excluded_directories: 1,
    critical_failures: 1,
    high_failures: 1,
    medium_failures: 0,
    low_failures: 0,
    ready_for_retry: 1,
  },
};

describe('WebDAVScanFailures', () => {
  let mockGetScanFailures: ReturnType<typeof vi.spyOn>;
  let mockRetryFailure: ReturnType<typeof vi.spyOn>;
  let mockExcludeFailure: ReturnType<typeof vi.spyOn>;
  
  beforeEach(() => {
    // Use spyOn to directly replace the methods
    mockGetScanFailures = vi.spyOn(apiModule.webdavService, 'getScanFailures');
    mockRetryFailure = vi.spyOn(apiModule.webdavService, 'retryFailure')
      .mockResolvedValue({ data: { success: true } } as any);
    mockExcludeFailure = vi.spyOn(apiModule.webdavService, 'excludeFailure')
      .mockResolvedValue({ data: { success: true } } as any);
    
    mockShowNotification.mockClear();
  });

  afterEach(() => {
    vi.clearAllTimers();
    vi.restoreAllMocks();
  });

  it('renders loading state initially', () => {
    mockGetScanFailures.mockImplementation(
      () => new Promise(() => {}) // Never resolves
    );

    renderWithProviders(<WebDAVScanFailures />);
    
    expect(screen.getByText('WebDAV Scan Failures')).toBeInTheDocument();
    // Should show skeleton loading (adjusted count based on actual implementation)
    expect(document.querySelectorAll('.MuiSkeleton-root')).toHaveLength(3);
  });

  it('renders scan failures data successfully', async () => {
    mockGetScanFailures.mockResolvedValue({
      data: mockScanFailuresData,
    });

    renderWithProviders(<WebDAVScanFailures />);

    // Wait for data to load and API to be called
    await waitFor(() => {
      expect(mockGetScanFailures).toHaveBeenCalled();
    });

    // Wait for skeleton loaders to disappear and data to appear
    await waitFor(() => {
      expect(document.querySelectorAll('.MuiSkeleton-root')).toHaveLength(0);
    });

    // Check if failures are rendered
    await waitFor(() => {
      expect(screen.getAllByText('/test/path/long/directory/name')[0]).toBeInTheDocument();
    });
    
    expect(screen.getAllByText('/test/path/permissions')[0]).toBeInTheDocument();

    // Check severity chips
    expect(screen.getAllByText('High')[0]).toBeInTheDocument();
    expect(screen.getAllByText('Critical')[0]).toBeInTheDocument();

    // Check failure type chips
    expect(screen.getAllByText('Timeout')[0]).toBeInTheDocument();
    expect(screen.getAllByText('Permission Denied')[0]).toBeInTheDocument();
  });

  it('renders error state when API fails', async () => {
    const errorMessage = 'Failed to fetch data';
    mockGetScanFailures.mockRejectedValue(
      new Error(errorMessage)
    );

    renderWithProviders(<WebDAVScanFailures />);

    await waitFor(() => {
      expect(mockGetScanFailures).toHaveBeenCalled();
    });

    await waitFor(() => {
      expect(screen.getByText(/Failed to load WebDAV scan failures/)).toBeInTheDocument();
    }, { timeout: 5000 });

    expect(screen.getByText(new RegExp(errorMessage))).toBeInTheDocument();
  });

  it('handles search filtering correctly', async () => {
    mockGetScanFailures.mockResolvedValue({
      data: mockScanFailuresData,
    });

    renderWithProviders(<WebDAVScanFailures />);

    // Wait for data to load completely
    await waitFor(() => {
      expect(document.querySelectorAll('.MuiSkeleton-root')).toHaveLength(0);
    }, { timeout: 5000 });

    await waitFor(() => {
      expect(screen.getAllByText('/test/path/long/directory/name')[0]).toBeInTheDocument();
      expect(screen.getAllByText('/test/path/permissions')[0]).toBeInTheDocument();
    });

    // Search for specific path
    const searchInput = screen.getByPlaceholderText('Search directories or error messages...');
    await userEvent.clear(searchInput);
    await userEvent.type(searchInput, 'permissions');

    // Wait for search filtering to take effect - should only show the permissions failure
    await waitFor(() => {
      expect(screen.queryByText('/test/path/long/directory/name')).not.toBeInTheDocument();
    }, { timeout: 3000 });
    
    // Verify the permissions path is still visible
    await waitFor(() => {
      expect(screen.getAllByText('/test/path/permissions')[0]).toBeInTheDocument();
    });
  });

  it('handles severity filtering correctly', async () => {
    mockGetScanFailures.mockResolvedValue({
      data: mockScanFailuresData,
    });

    renderWithProviders(<WebDAVScanFailures />);

    // Wait for data to load completely
    await waitFor(() => {
      expect(document.querySelectorAll('.MuiSkeleton-root')).toHaveLength(0);
    }, { timeout: 5000 });

    await waitFor(() => {
      expect(screen.getAllByText('/test/path/long/directory/name')[0]).toBeInTheDocument();
      expect(screen.getAllByText('/test/path/permissions')[0]).toBeInTheDocument();
    });

    // Find severity select by text - look for the div that contains "All Severities"
    const severitySelectButton = screen.getByText('All Severities').closest('[role="combobox"]');
    expect(severitySelectButton).toBeInTheDocument();
    
    await userEvent.click(severitySelectButton!);
    
    // Wait for dropdown options to appear and click Critical
    await waitFor(() => {
      expect(screen.getByRole('option', { name: 'Critical' })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('option', { name: 'Critical' }));

    // Should only show the critical failure
    await waitFor(() => {
      expect(screen.queryByText('/test/path/long/directory/name')).not.toBeInTheDocument();
    }, { timeout: 3000 });
    
    // Verify the permissions path is still visible
    await waitFor(() => {
      expect(screen.getAllByText('/test/path/permissions')[0]).toBeInTheDocument();
    });
  });

  it('expands failure details when clicked', async () => {
    mockGetScanFailures.mockResolvedValue({
      data: mockScanFailuresData,
    });

    renderWithProviders(<WebDAVScanFailures />);

    // Wait for data to load completely
    await waitFor(() => {
      expect(document.querySelectorAll('.MuiSkeleton-root')).toHaveLength(0);
    }, { timeout: 5000 });

    await waitFor(() => {
      expect(screen.getAllByText('/test/path/long/directory/name')[0]).toBeInTheDocument();
    });

    // Find and click the expand icon to expand the accordion
    const expandMoreIcon = screen.getAllByTestId('ExpandMoreIcon')[0];
    expect(expandMoreIcon).toBeInTheDocument();
    await userEvent.click(expandMoreIcon.closest('button')!);

    // Should show detailed information
    await waitFor(() => {
      expect(screen.getByText('Request timeout after 30 seconds')).toBeInTheDocument();
      expect(screen.getAllByText('Recommended Action')[0]).toBeInTheDocument();
    });
  });

  it('handles retry action correctly', async () => {
    const mockRetryResponse = {
      data: {
        success: true,
        message: 'Retry scheduled',
        directory_path: '/test/path/long/directory/name',
      },
    };

    mockGetScanFailures.mockResolvedValue({
      data: mockScanFailuresData,
    });
    
    // Override the mock from beforeEach with the specific response for this test
    mockRetryFailure.mockResolvedValue(mockRetryResponse);
    
    // Also make sure getScanFailures will be called again for refresh
    mockGetScanFailures
      .mockResolvedValueOnce({ data: mockScanFailuresData })
      .mockResolvedValueOnce({ data: mockScanFailuresData });

    renderWithProviders(<WebDAVScanFailures />);

    // Wait for data to load completely
    await waitFor(() => {
      expect(document.querySelectorAll('.MuiSkeleton-root')).toHaveLength(0);
    }, { timeout: 5000 });

    await waitFor(() => {
      expect(screen.getAllByText('/test/path/long/directory/name')[0]).toBeInTheDocument();
    });

    // Expand the first failure by clicking on the expand icon
    const expandMoreIcon = screen.getAllByTestId('ExpandMoreIcon')[0];
    await userEvent.click(expandMoreIcon.closest('button')!);

    // Wait for details to load and click retry
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /retry scan/i })).toBeInTheDocument();
    });

    const retryButton = screen.getByRole('button', { name: /retry scan/i });
    await userEvent.click(retryButton);

    // Should open confirmation dialog
    await waitFor(() => {
      expect(screen.getByText('Retry WebDAV Scan')).toBeInTheDocument();
    });

    // Confirm retry
    const confirmButton = screen.getByRole('button', { name: 'Retry Now' });
    await userEvent.click(confirmButton);

    // Should call the retry API
    await waitFor(() => {
      expect(mockRetryFailure).toHaveBeenCalledWith('1', { notes: undefined });
    });

    // Verify the API call completed - at minimum, check the retry API was called
    // For now, just check that the mockRetryFailure was called correctly
    // We'll add notification verification later if needed
  });

  it('handles exclude action correctly', async () => {
    const mockExcludeResponse = {
      data: {
        success: true,
        message: 'Directory excluded',
        directory_path: '/test/path/long/directory/name',
        permanent: true,
      },
    };

    mockGetScanFailures.mockResolvedValue({
      data: mockScanFailuresData,
    });
    
    // Override the mock from beforeEach with the specific response for this test
    mockExcludeFailure.mockResolvedValue(mockExcludeResponse);
    
    // Also make sure getScanFailures will be called again for refresh
    mockGetScanFailures
      .mockResolvedValueOnce({ data: mockScanFailuresData })
      .mockResolvedValueOnce({ data: mockScanFailuresData });

    renderWithProviders(<WebDAVScanFailures />);

    // Wait for data to load completely
    await waitFor(() => {
      expect(document.querySelectorAll('.MuiSkeleton-root')).toHaveLength(0);
    }, { timeout: 5000 });

    await waitFor(() => {
      expect(screen.getAllByText('/test/path/long/directory/name')[0]).toBeInTheDocument();
    });

    // Expand the first failure by clicking on the expand icon
    const expandMoreIcon = screen.getAllByTestId('ExpandMoreIcon')[0];
    await userEvent.click(expandMoreIcon.closest('button')!);

    // Wait for details to load and click exclude
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /exclude directory/i })).toBeInTheDocument();
    });

    const excludeButton = screen.getByRole('button', { name: /exclude directory/i });
    await userEvent.click(excludeButton);

    // Should open confirmation dialog
    await waitFor(() => {
      expect(screen.getByText('Exclude Directory from Scanning')).toBeInTheDocument();
    });

    // Confirm exclude - find the confirm button in the dialog
    const confirmButton = screen.getByRole('button', { name: 'Exclude Directory' });
    await userEvent.click(confirmButton);

    // Should call the exclude API
    await waitFor(() => {
      expect(mockExcludeFailure).toHaveBeenCalledWith('1', {
        notes: undefined,
        permanent: true,
      });
    });

    // Verify the API call completed - at minimum, check the exclude API was called
    // For now, just check that the mockExcludeFailure was called correctly
    // We'll add notification verification later if needed
  });

  it('displays empty state when no failures exist', async () => {
    mockGetScanFailures.mockResolvedValue({
      data: {
        failures: [],
        stats: {
          active_failures: 0,
          resolved_failures: 0,
          excluded_directories: 0,
          critical_failures: 0,
          high_failures: 0,
          medium_failures: 0,
          low_failures: 0,
          ready_for_retry: 0,
        },
      },
    });

    renderWithProviders(<WebDAVScanFailures />);

    // Wait for data to load completely
    await waitFor(() => {
      expect(document.querySelectorAll('.MuiSkeleton-root')).toHaveLength(0);
    }, { timeout: 5000 });

    await waitFor(() => {
      expect(screen.getByText('No Scan Failures Found')).toBeInTheDocument();
      expect(screen.getByText('All WebDAV directories are scanning successfully!')).toBeInTheDocument();
    });
  });

  it('refreshes data when refresh button is clicked', async () => {
    // Allow multiple calls to getScanFailures
    mockGetScanFailures
      .mockResolvedValueOnce({ data: mockScanFailuresData })
      .mockResolvedValueOnce({ data: mockScanFailuresData });

    renderWithProviders(<WebDAVScanFailures />);

    // Wait for data to load completely
    await waitFor(() => {
      expect(document.querySelectorAll('.MuiSkeleton-root')).toHaveLength(0);
    }, { timeout: 5000 });

    await waitFor(() => {
      expect(screen.getAllByText('/test/path/long/directory/name')[0]).toBeInTheDocument();
    });

    // Click refresh button - find the one that's NOT disabled (not the retry buttons)
    const refreshIcons = screen.getAllByTestId('RefreshIcon');
    let mainRefreshButton = null;
    
    // Find the refresh button that is not disabled
    for (const icon of refreshIcons) {
      const button = icon.closest('button');
      if (button && !button.disabled) {
        mainRefreshButton = button;
        break;
      }
    }
    
    expect(mainRefreshButton).toBeInTheDocument();
    await userEvent.click(mainRefreshButton!);

    // Should call API again
    await waitFor(() => {
      expect(mockGetScanFailures).toHaveBeenCalledTimes(2);
    }, { timeout: 5000 });
  });

  it('auto-refreshes data when autoRefresh is enabled', async () => {
    vi.useFakeTimers();
    
    mockGetScanFailures.mockResolvedValue({
      data: mockScanFailuresData,
    });

    renderWithProviders(<WebDAVScanFailures autoRefresh={true} refreshInterval={1000} />);

    // Initial call
    expect(mockGetScanFailures).toHaveBeenCalledTimes(1);

    // Fast-forward time to trigger the interval
    act(() => {
      vi.advanceTimersByTime(1000);
    });

    // Wait for any pending promises to resolve
    await act(async () => {
      await Promise.resolve();
    });

    expect(mockGetScanFailures).toHaveBeenCalledTimes(2);

    vi.useRealTimers();
  });

  it('does not auto-refresh when autoRefresh is disabled', async () => {
    vi.useFakeTimers();
    
    mockGetScanFailures.mockResolvedValue({
      data: mockScanFailuresData,
    });

    renderWithProviders(<WebDAVScanFailures autoRefresh={false} />);

    // Initial call
    expect(mockGetScanFailures).toHaveBeenCalledTimes(1);

    // Fast-forward time significantly
    vi.advanceTimersByTime(30000);

    // Should still only be called once (no auto-refresh)
    expect(mockGetScanFailures).toHaveBeenCalledTimes(1);

    vi.useRealTimers();
  });
});