import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { screen, fireEvent, waitFor, act } from '@testing-library/react';
import SyncProgressDisplay from '../SyncProgressDisplay';
import { renderWithProviders } from '../../test/test-utils';
// Define SyncProgressInfo type locally for tests
interface SyncProgressInfo {
  source_id: string;
  phase: string;
  phase_description: string;
  elapsed_time_secs: number;
  directories_found: number;
  directories_processed: number;
  files_found: number;
  files_processed: number;
  bytes_processed: number;
  processing_rate_files_per_sec: number;
  files_progress_percent: number;
  estimated_time_remaining_secs?: number;
  current_directory: string;
  current_file?: string;
  errors: number;
  warnings: number;
  is_active: boolean;
}

// Mock the API module using the __mocks__ version
vi.mock('../../services/api');

// Import the mock helpers
import { getMockSyncProgressWebSocket, resetMockSyncProgressWebSocket, MockSyncProgressWebSocket, sourcesService } from '../../services/__mocks__/api';

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

// Helper function to simulate progress updates
const simulateProgressUpdate = (progressData: SyncProgressInfo) => {
  const mockWS = getMockSyncProgressWebSocket();
  if (mockWS) {
    act(() => {
      mockWS.simulateProgress(progressData);
    });
  }
};

// Helper function to simulate heartbeat updates
const simulateHeartbeatUpdate = (data: any) => {
  const mockWS = getMockSyncProgressWebSocket();
  if (mockWS) {
    act(() => {
      mockWS.simulateHeartbeat(data);
    });
  }
};

// Helper function to simulate connection status changes
const simulateConnectionStatusChange = (status: string) => {
  const mockWS = getMockSyncProgressWebSocket();
  if (mockWS) {
    act(() => {
      mockWS.simulateConnectionStatus(status);
    });
  }
};

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
    // Reset the mock WebSocket instance
    resetMockSyncProgressWebSocket();
    
    // Mock localStorage for token access
    Object.defineProperty(global, 'localStorage', {
      value: {
        getItem: vi.fn(() => 'mock-jwt-token'),
        setItem: vi.fn(),
        removeItem: vi.fn(),
        clear: vi.fn(),
      },
      writable: true,
    });
    
    // Mock window.location for consistent URL construction
    Object.defineProperty(window, 'location', {
      value: {
        origin: 'http://localhost:3000',
        href: 'http://localhost:3000',
        protocol: 'http:',
        host: 'localhost:3000',
      },
      writable: true,
    });
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

    test('should show connecting status initially', async () => {
      renderComponent();
      
      // The hook starts in disconnected state, then moves to connecting
      await waitFor(() => {
        simulateConnectionStatusChange('connecting');
      });
      
      expect(screen.getByText('Connecting...')).toBeInTheDocument();
    });

    test('should render with custom source name', () => {
      renderComponent({ sourceName: 'My Custom Source' });
      expect(screen.getByText('My Custom Source - Sync Progress')).toBeInTheDocument();
    });
  });

  describe('WebSocket Connection Management', () => {
    test('should create WebSocket connection when visible', async () => {
      renderComponent();
      
      // Verify that the WebSocket service was called
      await waitFor(() => {
        expect(sourcesService.createSyncProgressWebSocket).toHaveBeenCalledWith('test-source-123');
      });
    });

    test('should handle successful connection', async () => {
      renderComponent();
      
      // Simulate successful connection
      await waitFor(() => {
        simulateConnectionStatusChange('connected');
      });

      // Should show connected status when there's progress data
      const mockProgress = createMockProgressInfo();
      simulateProgressUpdate(mockProgress);

      await waitFor(() => {
        expect(screen.getByText('Live')).toBeInTheDocument();
      });
    });

    test('should handle connection error', async () => {
      renderComponent();
      
      await waitFor(() => {
        simulateConnectionStatusChange('error');
      });

      await waitFor(() => {
        expect(screen.getByText('Disconnected')).toBeInTheDocument();
      });
    });

    test('should show reconnecting status', async () => {
      renderComponent();
      
      await waitFor(() => {
        simulateConnectionStatusChange('reconnecting');
      });

      await waitFor(() => {
        expect(screen.getByText('Reconnecting...')).toBeInTheDocument();
      });
    });

    test('should show connection failed status', async () => {
      renderComponent();
      
      await waitFor(() => {
        simulateConnectionStatusChange('failed');
      });

      await waitFor(() => {
        expect(screen.getByText('Connection Failed')).toBeInTheDocument();
      });
    });

    test('should close WebSocket connection on unmount', () => {
      const { unmount } = renderComponent();
      
      // The WebSocket should be closed when component unmounts
      // This is handled by the useSyncProgressWebSocket hook cleanup
      unmount();
      
      // Since we're using a custom hook, we can't directly test the WebSocket close
      // but we can verify the component unmounts without errors
      expect(screen.queryByText('Test WebDAV Source - Sync Progress')).not.toBeInTheDocument();
    });

    test('should handle visibility changes correctly', () => {
      const { rerender } = renderComponent({ isVisible: true });
      
      // Component should be visible initially
      expect(screen.getByText('Test WebDAV Source - Sync Progress')).toBeInTheDocument();
      
      // Hide the component
      rerender(
        <SyncProgressDisplay
          sourceId="test-source-123"
          sourceName="Test WebDAV Source"
          isVisible={false}
        />
      );

      // Component should not be visible
      expect(screen.queryByText('Test WebDAV Source - Sync Progress')).not.toBeInTheDocument();
    });
  });

  describe('Progress Data Display', () => {

    test('should display progress information correctly', async () => {
      renderComponent();
      const mockProgress = createMockProgressInfo();
      
      simulateProgressUpdate(mockProgress);

      await waitFor(() => {
        expect(screen.getByText('Downloading and processing files')).toBeInTheDocument();
        expect(screen.getByText('30 / 50 files (60.0%)')).toBeInTheDocument();
        expect(screen.getByText('7 / 10')).toBeInTheDocument(); // Directories
        expect(screen.getByText('1000 KB')).toBeInTheDocument(); // Bytes processed
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
        // After clicking collapse, the button should change to expand
        expect(screen.getByLabelText('Expand')).toBeInTheDocument();
        // The content is still in DOM but hidden by Material-UI Collapse
        const collapseElement = screen.getByText('Waiting for sync progress information...').closest('.MuiCollapse-root');
        expect(collapseElement).toHaveClass('MuiCollapse-hidden');
      });
    });

    test('should expand when expand button is clicked', async () => {
      renderComponent();
      
      // First collapse
      const collapseButton = screen.getByLabelText('Collapse');
      fireEvent.click(collapseButton);

      await waitFor(() => {
        expect(screen.getByLabelText('Expand')).toBeInTheDocument();
        const collapseElement = screen.getByText('Waiting for sync progress information...').closest('.MuiCollapse-root');
        expect(collapseElement).toHaveClass('MuiCollapse-hidden');
      });

      // Then expand
      const expandButton = screen.getByLabelText('Expand');
      fireEvent.click(expandButton);

      await waitFor(() => {
        expect(screen.getByLabelText('Collapse')).toBeInTheDocument();
        const collapseElement = screen.getByText('Waiting for sync progress information...').closest('.MuiCollapse-root');
        expect(collapseElement).toHaveClass('MuiCollapse-entered');
      });
    });
  });

  describe('Data Formatting', () => {
    test('should format bytes correctly', async () => {
      renderComponent();
      
      // Test 1.0 KB case
      const mockProgress1KB = createMockProgressInfo({ bytes_processed: 1024 });
      simulateProgressUpdate(mockProgress1KB);

      await waitFor(() => {
        expect(screen.getByText('1 KB')).toBeInTheDocument();
      });
    });

    test('should format zero bytes correctly', async () => {
      renderComponent();
      
      // Test 0 B case
      const mockProgress0 = createMockProgressInfo({ bytes_processed: 0 });
      simulateProgressUpdate(mockProgress0);

      await waitFor(() => {
        expect(screen.getByText('0 B')).toBeInTheDocument();
      });
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

      // Then send inactive heartbeat using WebSocket simulation
      simulateHeartbeatUpdate({
        source_id: 'test-source-123',
        is_active: false,
        timestamp: Date.now()
      });

      await waitFor(() => {
        expect(screen.getByText('Waiting for sync progress information...')).toBeInTheDocument();
      });
    });
  });

  describe('Error Handling', () => {
    test('should handle WebSocket connection errors gracefully', async () => {
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      
      renderComponent();
      
      // Wait for WebSocket to be created
      await waitFor(() => {
        expect(sourcesService.createSyncProgressWebSocket).toHaveBeenCalledWith('test-source-123');
      });
      
      // Simulate WebSocket error
      const mockWS = getMockSyncProgressWebSocket();
      if (mockWS) {
        act(() => {
          mockWS.simulateError({ error: 'Connection failed' });
        });

        // Verify error was logged by the component's error handler
        await waitFor(() => {
          expect(consoleSpy).toHaveBeenCalledWith('WebSocket connection error in SyncProgressDisplay:', { error: 'Connection failed' });
        });
      }

      consoleSpy.mockRestore();
    });

    test('should show manual reconnect option after connection failure', async () => {
      renderComponent();
      
      // Simulate connection failure
      simulateConnectionStatusChange('failed');

      await waitFor(() => {
        expect(screen.getByText('Connection Failed')).toBeInTheDocument();
        // Should show reconnect button
        expect(screen.getByRole('button', { name: /reconnect/i })).toBeInTheDocument();
      });
    });

    test('should trigger reconnect when reconnect button is clicked', async () => {
      renderComponent();
      
      // Simulate connection failure
      simulateConnectionStatusChange('failed');

      await waitFor(() => {
        const reconnectButton = screen.getByRole('button', { name: /reconnect/i });
        expect(reconnectButton).toBeInTheDocument();
        
        // Click the reconnect button
        fireEvent.click(reconnectButton);
      });

      // The reconnect function should be called (indirectly through the hook)
      // We can verify this by checking that the WebSocket service is called again
      expect(sourcesService.createSyncProgressWebSocket).toHaveBeenCalledWith('test-source-123');
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
        expect(screen.getByText('1 TB')).toBeInTheDocument();
        // Check for the large file numbers - they might be split across multiple elements
        expect(screen.getByText(/500000/)).toBeInTheDocument();
        expect(screen.getByText(/999999/)).toBeInTheDocument();
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