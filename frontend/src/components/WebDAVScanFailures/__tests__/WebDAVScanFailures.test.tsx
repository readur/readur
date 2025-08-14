import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import { ThemeProvider } from '@mui/material/styles';

import WebDAVScanFailures from '../WebDAVScanFailures';
import { webdavService } from '../../../services/api';
import { NotificationContext } from '../../../contexts/NotificationContext';
import theme from '../../../theme';

// Mock the webdav service
vi.mock('../../../services/api', () => ({
  webdavService: {
    getScanFailures: vi.fn(),
    retryFailure: vi.fn(),
    excludeFailure: vi.fn(),
  },
}));

const mockShowNotification = vi.fn();

const MockNotificationProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <NotificationContext.Provider value={{ showNotification: mockShowNotification }}>
    {children}
  </NotificationContext.Provider>
);

const renderWithProviders = (component: React.ReactElement) => {
  return render(
    <ThemeProvider theme={theme}>
      <MockNotificationProvider>
        {component}
      </MockNotificationProvider>
    </ThemeProvider>
  );
};

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
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.clearAllTimers();
  });

  it('renders loading state initially', () => {
    vi.mocked(webdavService.getScanFailures).mockImplementation(
      () => new Promise(() => {}) // Never resolves
    );

    renderWithProviders(<WebDAVScanFailures />);
    
    expect(screen.getByText('WebDAV Scan Failures')).toBeInTheDocument();
    // Should show skeleton loading
    expect(document.querySelectorAll('.MuiSkeleton-root')).toHaveLength(6); // Stats dashboard skeletons
  });

  it('renders scan failures data successfully', async () => {
    vi.mocked(webdavService.getScanFailures).mockResolvedValue({
      data: mockScanFailuresData,
    } as any);

    renderWithProviders(<WebDAVScanFailures />);

    await waitFor(() => {
      expect(screen.getByText('WebDAV Scan Failures')).toBeInTheDocument();
    });

    // Check if failures are rendered
    expect(screen.getByText('/test/path/long/directory/name')).toBeInTheDocument();
    expect(screen.getByText('/test/path/permissions')).toBeInTheDocument();

    // Check severity chips
    expect(screen.getByText('High')).toBeInTheDocument();
    expect(screen.getByText('Critical')).toBeInTheDocument();

    // Check failure type chips
    expect(screen.getByText('Timeout')).toBeInTheDocument();
    expect(screen.getByText('Permission Denied')).toBeInTheDocument();
  });

  it('renders error state when API fails', async () => {
    const errorMessage = 'Failed to fetch data';
    vi.mocked(webdavService.getScanFailures).mockRejectedValue(
      new Error(errorMessage)
    );

    renderWithProviders(<WebDAVScanFailures />);

    await waitFor(() => {
      expect(screen.getByText(/Failed to load WebDAV scan failures/)).toBeInTheDocument();
    });

    expect(screen.getByText(new RegExp(errorMessage))).toBeInTheDocument();
  });

  it('handles search filtering correctly', async () => {
    vi.mocked(webdavService.getScanFailures).mockResolvedValue({
      data: mockScanFailuresData,
    } as any);

    renderWithProviders(<WebDAVScanFailures />);

    await waitFor(() => {
      expect(screen.getByText('/test/path/long/directory/name')).toBeInTheDocument();
    });

    // Search for specific path
    const searchInput = screen.getByPlaceholderText('Search directories or error messages...');
    await userEvent.type(searchInput, 'permissions');

    // Should only show the permissions failure
    expect(screen.queryByText('/test/path/long/directory/name')).not.toBeInTheDocument();
    expect(screen.getByText('/test/path/permissions')).toBeInTheDocument();
  });

  it('handles severity filtering correctly', async () => {
    vi.mocked(webdavService.getScanFailures).mockResolvedValue({
      data: mockScanFailuresData,
    } as any);

    renderWithProviders(<WebDAVScanFailures />);

    await waitFor(() => {
      expect(screen.getByText('/test/path/long/directory/name')).toBeInTheDocument();
    });

    // Filter by critical severity
    const severitySelect = screen.getByLabelText('Severity');
    fireEvent.mouseDown(severitySelect);
    await userEvent.click(screen.getByText('Critical'));

    // Should only show the critical failure
    expect(screen.queryByText('/test/path/long/directory/name')).not.toBeInTheDocument();
    expect(screen.getByText('/test/path/permissions')).toBeInTheDocument();
  });

  it('expands failure details when clicked', async () => {
    vi.mocked(webdavService.getScanFailures).mockResolvedValue({
      data: mockScanFailuresData,
    } as any);

    renderWithProviders(<WebDAVScanFailures />);

    await waitFor(() => {
      expect(screen.getByText('/test/path/long/directory/name')).toBeInTheDocument();
    });

    // Click on the first failure to expand it
    const firstFailure = screen.getByText('/test/path/long/directory/name');
    await userEvent.click(firstFailure);

    // Should show detailed information
    await waitFor(() => {
      expect(screen.getByText('Request timeout after 30 seconds')).toBeInTheDocument();
      expect(screen.getByText('Recommended Action')).toBeInTheDocument();
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

    vi.mocked(webdavService.getScanFailures).mockResolvedValue({
      data: mockScanFailuresData,
    } as any);
    vi.mocked(webdavService.retryFailure).mockResolvedValue(mockRetryResponse as any);

    renderWithProviders(<WebDAVScanFailures />);

    await waitFor(() => {
      expect(screen.getByText('/test/path/long/directory/name')).toBeInTheDocument();
    });

    // Expand the first failure
    const firstFailure = screen.getByText('/test/path/long/directory/name');
    await userEvent.click(firstFailure);

    // Wait for details to load and click retry
    await waitFor(() => {
      expect(screen.getByText('Retry Scan')).toBeInTheDocument();
    });

    const retryButton = screen.getByText('Retry Scan');
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
      expect(webdavService.retryFailure).toHaveBeenCalledWith('1', { notes: undefined });
    });

    // Should show success notification
    expect(mockShowNotification).toHaveBeenCalledWith({
      type: 'success',
      message: 'Retry scheduled for: /test/path/long/directory/name',
    });
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

    vi.mocked(webdavService.getScanFailures).mockResolvedValue({
      data: mockScanFailuresData,
    } as any);
    vi.mocked(webdavService.excludeFailure).mockResolvedValue(mockExcludeResponse as any);

    renderWithProviders(<WebDAVScanFailures />);

    await waitFor(() => {
      expect(screen.getByText('/test/path/long/directory/name')).toBeInTheDocument();
    });

    // Expand the first failure
    const firstFailure = screen.getByText('/test/path/long/directory/name');
    await userEvent.click(firstFailure);

    // Wait for details to load and click exclude
    await waitFor(() => {
      expect(screen.getByText('Exclude Directory')).toBeInTheDocument();
    });

    const excludeButton = screen.getByText('Exclude Directory');
    await userEvent.click(excludeButton);

    // Should open confirmation dialog
    await waitFor(() => {
      expect(screen.getByText('Exclude Directory from Scanning')).toBeInTheDocument();
    });

    // Confirm exclude
    const confirmButton = screen.getByRole('button', { name: 'Exclude Directory' });
    await userEvent.click(confirmButton);

    // Should call the exclude API
    await waitFor(() => {
      expect(webdavService.excludeFailure).toHaveBeenCalledWith('1', {
        notes: undefined,
        permanent: true,
      });
    });

    // Should show success notification
    expect(mockShowNotification).toHaveBeenCalledWith({
      type: 'success',
      message: 'Directory excluded: /test/path/long/directory/name',
    });
  });

  it('displays empty state when no failures exist', async () => {
    vi.mocked(webdavService.getScanFailures).mockResolvedValue({
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
    } as any);

    renderWithProviders(<WebDAVScanFailures />);

    await waitFor(() => {
      expect(screen.getByText('No Scan Failures Found')).toBeInTheDocument();
      expect(screen.getByText('All WebDAV directories are scanning successfully!')).toBeInTheDocument();
    });
  });

  it('refreshes data when refresh button is clicked', async () => {
    vi.mocked(webdavService.getScanFailures).mockResolvedValue({
      data: mockScanFailuresData,
    } as any);

    renderWithProviders(<WebDAVScanFailures />);

    await waitFor(() => {
      expect(screen.getByText('/test/path/long/directory/name')).toBeInTheDocument();
    });

    // Click refresh button
    const refreshButton = screen.getByRole('button', { name: '' }); // IconButton without accessible name
    await userEvent.click(refreshButton);

    // Should call API again
    expect(webdavService.getScanFailures).toHaveBeenCalledTimes(2);
  });

  it('auto-refreshes data when autoRefresh is enabled', async () => {
    vi.useFakeTimers();
    
    vi.mocked(webdavService.getScanFailures).mockResolvedValue({
      data: mockScanFailuresData,
    } as any);

    renderWithProviders(<WebDAVScanFailures autoRefresh={true} refreshInterval={1000} />);

    await waitFor(() => {
      expect(webdavService.getScanFailures).toHaveBeenCalledTimes(1);
    });

    // Fast-forward time
    vi.advanceTimersByTime(1000);

    await waitFor(() => {
      expect(webdavService.getScanFailures).toHaveBeenCalledTimes(2);
    });

    vi.useRealTimers();
  });

  it('does not auto-refresh when autoRefresh is disabled', async () => {
    vi.useFakeTimers();
    
    vi.mocked(webdavService.getScanFailures).mockResolvedValue({
      data: mockScanFailuresData,
    } as any);

    renderWithProviders(<WebDAVScanFailures autoRefresh={false} />);

    await waitFor(() => {
      expect(webdavService.getScanFailures).toHaveBeenCalledTimes(1);
    });

    // Fast-forward time
    vi.advanceTimersByTime(30000);

    // Should still only be called once
    expect(webdavService.getScanFailures).toHaveBeenCalledTimes(1);

    vi.useRealTimers();
  });
});