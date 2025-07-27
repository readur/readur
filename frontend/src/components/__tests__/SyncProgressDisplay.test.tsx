import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { screen, fireEvent, waitFor, act } from '@testing-library/react';
import SyncProgressDisplay from '../SyncProgressDisplay';
import { renderWithProviders } from '../../test/test-utils';
import type { SyncProgressInfo } from '../../services/api';

// Mock EventSource constants first
const EVENTSOURCE_CONNECTING = 0;
const EVENTSOURCE_OPEN = 1;
const EVENTSOURCE_CLOSED = 2;

// Mock EventSource globally
const mockEventSource = {
  onopen: null as ((event: Event) => void) | null,
  onmessage: null as ((event: MessageEvent) => void) | null,
  onerror: null as ((event: Event) => void) | null,
  addEventListener: vi.fn(),
  removeEventListener: vi.fn(),
  close: vi.fn(),
  readyState: EVENTSOURCE_CONNECTING,
  url: '',
  withCredentials: false,
  CONNECTING: EVENTSOURCE_CONNECTING,
  OPEN: EVENTSOURCE_OPEN,
  CLOSED: EVENTSOURCE_CLOSED,
  dispatchEvent: vi.fn(),
};

// Mock the global EventSource constructor
global.EventSource = vi.fn(() => mockEventSource) as any;
(global.EventSource as any).CONNECTING = EVENTSOURCE_CONNECTING;
(global.EventSource as any).OPEN = EVENTSOURCE_OPEN;
(global.EventSource as any).CLOSED = EVENTSOURCE_CLOSED;

// Mock the sourcesService
const mockSourcesService = {
  getSyncProgressStream: vi.fn(() => {
    // Create a new mock for each call to simulate real EventSource behavior
    return {
      ...mockEventSource,
      addEventListener: vi.fn(),
      close: vi.fn(),
    };
  }),
  triggerSync: vi.fn(),
  stopSync: vi.fn(),
  getSyncStatus: vi.fn(),
  triggerDeepScan: vi.fn(),
};

vi.mock('../../services/api', async () => {
  const actual = await vi.importActual('../../services/api');
  return {
    ...actual,
    sourcesService: mockSourcesService,
  };
});

// Create mock progress data factory
const createMockProgressInfo = (overrides: Partial<SyncProgressInfo> = {}): SyncProgressInfo => ({
  source_id: 'test-source-123',
  phase: 'processing_files',
  phase_description: 'Downloading and processing files',
  elapsed_time_secs: 120,
  directories_found: 10,
  directories_processed: 7,
  files_found: 50,
  files_processed: 30,
  bytes_processed: 1024000,
  processing_rate_files_per_sec: 2.5,
  files_progress_percent: 60.0,
  estimated_time_remaining_secs: 80,
  current_directory: '/Documents/Projects',
  current_file: 'important-document.pdf',
  errors: 0,
  warnings: 1,
  is_active: true,
  ...overrides,
});

const renderComponent = (props: Partial<React.ComponentProps<typeof SyncProgressDisplay>> = {}) => {
  const defaultProps = {
    sourceId: 'test-source-123',
    sourceName: 'Test WebDAV Source',
    isVisible: true,
    ...props,
  };

  return renderWithProviders(<SyncProgressDisplay {...defaultProps} />);
};

describe('SyncProgressDisplay Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockEventSource.close.mockClear();
    mockEventSource.addEventListener.mockClear();
    mockEventSource.onopen = null;
    mockEventSource.onmessage = null;
    mockEventSource.onerror = null;
    mockEventSource.readyState = EVENTSOURCE_CONNECTING;
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('Visibility and Rendering', () => {
    test('should not render when isVisible is false', () => {
      renderComponent({ isVisible: false });
      expect(screen.queryByText('Test WebDAV Source - Sync Progress')).not.toBeInTheDocument();
    });

    test('should render when isVisible is true', () => {
      renderComponent({ isVisible: true });
      expect(screen.getByText('Test WebDAV Source - Sync Progress')).toBeInTheDocument();
    });

    test('should show connecting status initially', () => {
      renderComponent();
      expect(screen.getByText('Connecting...')).toBeInTheDocument();
    });

    test('should render with custom source name', () => {
      renderComponent({ sourceName: 'My Custom Source' });
      expect(screen.getByText('My Custom Source - Sync Progress')).toBeInTheDocument();
    });
  });

  describe('SSE Connection Management', () => {
    test('should create EventSource with correct URL', () => {
      renderComponent();
      expect(mockSourcesService.getSyncProgressStream).toHaveBeenCalledWith('test-source-123');
    });

    test('should handle successful connection', async () => {
      renderComponent();
      
      // Simulate successful connection
      act(() => {
        if (mockEventSource.onopen) {
          mockEventSource.onopen(new Event('open'));
        }
      });

      // Should show connected status when there's progress data
      const mockProgress = createMockProgressInfo();
      act(() => {
        if (mockEventSource.addEventListener.mock.calls.length > 0) {
          const progressHandler = mockEventSource.addEventListener.mock.calls.find(
            call => call[0] === 'progress'
          )?.[1] as (event: MessageEvent) => void;
          
          if (progressHandler) {
            progressHandler(new MessageEvent('progress', {
              data: JSON.stringify(mockProgress)
            }));
          }
        }
      });

      await waitFor(() => {
        expect(screen.getByText('Live')).toBeInTheDocument();
      });
    });

    test('should handle connection error', async () => {
      renderComponent();
      
      act(() => {
        if (mockEventSource.onerror) {
          mockEventSource.onerror(new Event('error'));
        }
      });

      await waitFor(() => {
        expect(screen.getByText('Disconnected')).toBeInTheDocument();
      });
    });

    test('should close EventSource on unmount', () => {
      const { unmount } = renderComponent();
      unmount();
      expect(mockEventSource.close).toHaveBeenCalled();
    });

    test('should close EventSource when visibility changes to false', () => {
      const { rerender } = renderComponent({ isVisible: true });
      
      rerender(
        <SyncProgressDisplay
          sourceId="test-source-123"
          sourceName="Test WebDAV Source"
          isVisible={false}
        />
      );

      expect(mockEventSource.close).toHaveBeenCalled();
    });
  });

  describe('Progress Data Display', () => {
    const simulateProgressUpdate = (progressData: SyncProgressInfo) => {
      act(() => {
        const progressHandler = mockEventSource.addEventListener.mock.calls.find(
          call => call[0] === 'progress'
        )?.[1] as (event: MessageEvent) => void;
        
        if (progressHandler) {
          progressHandler(new MessageEvent('progress', {
            data: JSON.stringify(progressData)
          }));
        }
      });
    };

    test('should display progress information correctly', async () => {
      renderComponent();
      const mockProgress = createMockProgressInfo();
      
      simulateProgressUpdate(mockProgress);

      await waitFor(() => {
        expect(screen.getByText('Downloading and processing files')).toBeInTheDocument();
        expect(screen.getByText('30 / 50 files (60.0%)')).toBeInTheDocument();
        expect(screen.getByText('7 / 10')).toBeInTheDocument(); // Directories
        expect(screen.getByText('1.0 MB')).toBeInTheDocument(); // Bytes processed
        expect(screen.getByText('2.5 files/sec')).toBeInTheDocument(); // Processing rate
        expect(screen.getByText('2m 0s')).toBeInTheDocument(); // Elapsed time
      });
    });

    test('should display current directory and file', async () => {
      renderComponent();
      const mockProgress = createMockProgressInfo({
        current_directory: '/Documents/Important',
        current_file: 'presentation.pptx',
      });
      
      simulateProgressUpdate(mockProgress);

      await waitFor(() => {
        expect(screen.getByText('/Documents/Important')).toBeInTheDocument();
        expect(screen.getByText('presentation.pptx')).toBeInTheDocument();
      });
    });

    test('should display estimated time remaining', async () => {
      renderComponent();
      const mockProgress = createMockProgressInfo({
        estimated_time_remaining_secs: 300, // 5 minutes
      });
      
      simulateProgressUpdate(mockProgress);

      await waitFor(() => {
        expect(screen.getByText(/Estimated time remaining: 5m 0s/)).toBeInTheDocument();
      });
    });

    test('should display errors and warnings', async () => {
      renderComponent();
      const mockProgress = createMockProgressInfo({
        errors: 3,
        warnings: 5,
      });
      
      simulateProgressUpdate(mockProgress);

      await waitFor(() => {
        expect(screen.getByText('3 errors')).toBeInTheDocument();
        expect(screen.getByText('5 warnings')).toBeInTheDocument();
      });
    });

    test('should handle singular error/warning labels', async () => {
      renderComponent();
      const mockProgress = createMockProgressInfo({
        errors: 1,
        warnings: 1,
      });
      
      simulateProgressUpdate(mockProgress);

      await waitFor(() => {
        expect(screen.getByText('1 error')).toBeInTheDocument();
        expect(screen.getByText('1 warning')).toBeInTheDocument();
      });
    });

    test('should not show errors/warnings when count is zero', async () => {
      renderComponent();
      const mockProgress = createMockProgressInfo({
        errors: 0,
        warnings: 0,
      });
      
      simulateProgressUpdate(mockProgress);

      await waitFor(() => {
        expect(screen.queryByText(/error/)).not.toBeInTheDocument();
        expect(screen.queryByText(/warning/)).not.toBeInTheDocument();
      });
    });

    test('should not show estimated time when not available', async () => {
      renderComponent();
      const mockProgress = createMockProgressInfo({
        estimated_time_remaining_secs: undefined,
      });
      
      simulateProgressUpdate(mockProgress);

      await waitFor(() => {
        expect(screen.queryByText(/Estimated time remaining/)).not.toBeInTheDocument();
      });
    });
  });

  describe('Phase Indicators and Colors', () => {
    const testPhases = [
      { phase: 'initializing', color: 'info', description: 'Initializing sync operation' },
      { phase: 'discovering_files', color: 'warning', description: 'Discovering files to sync' },
      { phase: 'processing_files', color: 'primary', description: 'Downloading and processing files' },
      { phase: 'completed', color: 'success', description: 'Sync completed successfully' },
      { phase: 'failed', color: 'error', description: 'Sync failed: Connection timeout' },
    ];

    testPhases.forEach(({ phase, description }) => {
      test(`should display correct phase description for ${phase}`, async () => {
        renderComponent();
        const mockProgress = createMockProgressInfo({
          phase,
          phase_description: description,
        });
        
        simulateProgressUpdate(mockProgress);

        await waitFor(() => {
          expect(screen.getByText(description)).toBeInTheDocument();
        });
      });
    });
  });

  describe('Progress Bar', () => {
    test('should show progress bar when files are found', async () => {
      renderComponent();
      const mockProgress = createMockProgressInfo({
        files_found: 100,
        files_processed: 75,
        files_progress_percent: 75.0,
      });
      
      simulateProgressUpdate(mockProgress);

      await waitFor(() => {
        const progressBar = screen.getByRole('progressbar');
        expect(progressBar).toBeInTheDocument();
        expect(progressBar).toHaveAttribute('aria-valuenow', '75');
      });
    });

    test('should not show progress bar when no files found', async () => {
      renderComponent();
      const mockProgress = createMockProgressInfo({
        files_found: 0,
        files_processed: 0,
      });
      
      simulateProgressUpdate(mockProgress);

      await waitFor(() => {
        expect(screen.queryByRole('progressbar')).not.toBeInTheDocument();
      });
    });
  });

  describe('Expand/Collapse Functionality', () => {
    test('should be expanded by default', () => {
      renderComponent();
      expect(screen.getByText('Waiting for sync progress information...')).toBeInTheDocument();
    });

    test('should collapse when collapse button is clicked', async () => {
      renderComponent();
      
      const collapseButton = screen.getByLabelText('Collapse');
      fireEvent.click(collapseButton);

      await waitFor(() => {
        expect(screen.queryByText('Waiting for sync progress information...')).not.toBeInTheDocument();
      });
    });

    test('should expand when expand button is clicked', async () => {
      renderComponent();
      
      // First collapse
      const collapseButton = screen.getByLabelText('Collapse');
      fireEvent.click(collapseButton);

      await waitFor(() => {
        expect(screen.queryByText('Waiting for sync progress information...')).not.toBeInTheDocument();
      });

      // Then expand
      const expandButton = screen.getByLabelText('Expand');
      fireEvent.click(expandButton);

      await waitFor(() => {
        expect(screen.getByText('Waiting for sync progress information...')).toBeInTheDocument();
      });
    });
  });

  describe('Data Formatting', () => {
    test('should format bytes correctly', async () => {
      renderComponent();
      
      const testCases = [
        { bytes: 0, expected: '0 B' },
        { bytes: 512, expected: '512 B' },
        { bytes: 1024, expected: '1.0 KB' },
        { bytes: 1536, expected: '1.5 KB' },
        { bytes: 1048576, expected: '1.0 MB' },
        { bytes: 1073741824, expected: '1.0 GB' },
      ];

      for (const { bytes, expected } of testCases) {
        const mockProgress = createMockProgressInfo({ bytes_processed: bytes });
        simulateProgressUpdate(mockProgress);

        await waitFor(() => {
          expect(screen.getByText(expected)).toBeInTheDocument();
        });
      }
    });

    test('should format duration correctly', async () => {
      renderComponent();
      
      const testCases = [
        { seconds: 30, expected: '30s' },
        { seconds: 90, expected: '1m 30s' },
        { seconds: 3661, expected: '1h 1m' },
      ];

      for (const { seconds, expected } of testCases) {
        const mockProgress = createMockProgressInfo({ elapsed_time_secs: seconds });
        simulateProgressUpdate(mockProgress);

        await waitFor(() => {
          expect(screen.getByText(expected)).toBeInTheDocument();
        });
      }
    });
  });

  describe('Heartbeat Handling', () => {
    test('should clear progress info on inactive heartbeat', async () => {
      renderComponent();
      
      // First set some progress
      const mockProgress = createMockProgressInfo();
      simulateProgressUpdate(mockProgress);

      await waitFor(() => {
        expect(screen.getByText('Downloading and processing files')).toBeInTheDocument();
      });

      // Then send inactive heartbeat
      act(() => {
        const heartbeatHandler = mockEventSource.addEventListener.mock.calls.find(
          call => call[0] === 'heartbeat'
        )?.[1] as (event: MessageEvent) => void;
        
        if (heartbeatHandler) {
          heartbeatHandler(new MessageEvent('heartbeat', {
            data: JSON.stringify({
              source_id: 'test-source-123',
              is_active: false,
              timestamp: Date.now()
            })
          }));
        }
      });

      await waitFor(() => {
        expect(screen.getByText('Waiting for sync progress information...')).toBeInTheDocument();
      });
    });
  });

  describe('Error Handling', () => {
    test('should handle malformed progress data gracefully', async () => {
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      
      renderComponent();
      
      act(() => {
        const progressHandler = mockEventSource.addEventListener.mock.calls.find(
          call => call[0] === 'progress'
        )?.[1] as (event: MessageEvent) => void;
        
        if (progressHandler) {
          progressHandler(new MessageEvent('progress', {
            data: 'invalid json'
          }));
        }
      });

      await waitFor(() => {
        expect(consoleSpy).toHaveBeenCalledWith('Failed to parse progress event:', expect.any(Error));
      });

      consoleSpy.mockRestore();
    });

    test('should handle malformed heartbeat data gracefully', async () => {
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      
      renderComponent();
      
      act(() => {
        const heartbeatHandler = mockEventSource.addEventListener.mock.calls.find(
          call => call[0] === 'heartbeat'
        )?.[1] as (event: MessageEvent) => void;
        
        if (heartbeatHandler) {
          heartbeatHandler(new MessageEvent('heartbeat', {
            data: 'invalid json'
          }));
        }
      });

      await waitFor(() => {
        expect(consoleSpy).toHaveBeenCalledWith('Failed to parse heartbeat event:', expect.any(Error));
      });

      consoleSpy.mockRestore();
    });
  });

  describe('Edge Cases', () => {
    test('should handle missing current_file gracefully', async () => {
      renderComponent();
      const mockProgress = createMockProgressInfo({
        current_directory: '/Documents',
        current_file: undefined,
      });
      
      simulateProgressUpdate(mockProgress);

      await waitFor(() => {
        expect(screen.getByText('/Documents')).toBeInTheDocument();
        expect(screen.queryByText('Current File')).not.toBeInTheDocument();
      });
    });

    test('should handle zero processing rate', async () => {
      renderComponent();
      const mockProgress = createMockProgressInfo({
        processing_rate_files_per_sec: 0.0,
      });
      
      simulateProgressUpdate(mockProgress);

      await waitFor(() => {
        expect(screen.getByText('0.0 files/sec')).toBeInTheDocument();
      });
    });

    test('should handle very large numbers', async () => {
      renderComponent();
      const mockProgress = createMockProgressInfo({
        bytes_processed: 1099511627776, // 1 TB
        files_found: 999999,
        files_processed: 500000,
      });
      
      simulateProgressUpdate(mockProgress);

      await waitFor(() => {
        expect(screen.getByText('1.0 TB')).toBeInTheDocument();
        expect(screen.getByText('500000 / 999999 files')).toBeInTheDocument();
      });
    });
  });

  describe('Accessibility', () => {
    test('should have proper ARIA labels', () => {
      renderComponent();
      
      expect(screen.getByLabelText('Collapse')).toBeInTheDocument();
    });

    test('should have accessible progress bar', async () => {
      renderComponent();
      const mockProgress = createMockProgressInfo();
      
      simulateProgressUpdate(mockProgress);

      await waitFor(() => {
        const progressBar = screen.getByRole('progressbar');
        expect(progressBar).toHaveAttribute('aria-valuenow', '60');
        expect(progressBar).toHaveAttribute('aria-valuemin', '0');
        expect(progressBar).toHaveAttribute('aria-valuemax', '100');
      });
    });
  });
});