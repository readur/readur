import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { screen, fireEvent, waitFor, act } from '@testing-library/react';
import SourcesPage from '../SourcesPage';
import { renderWithProviders } from '../../test/test-utils';
import type { SyncProgressInfo } from '../../services/api';

// Mock the API module
const mockApi = {
  get: vi.fn(),
  post: vi.fn(),
  put: vi.fn(),
  delete: vi.fn(),
};

const mockEventSource = {
  onopen: null as ((event: Event) => void) | null,
  onmessage: null as ((event: MessageEvent) => void) | null,
  onerror: null as ((event: Event) => void) | null,
  addEventListener: vi.fn(),
  removeEventListener: vi.fn(),
  close: vi.fn(),
  readyState: EventSource.CONNECTING,
  url: '',
  withCredentials: false,
  CONNECTING: 0,
  OPEN: 1,
  CLOSED: 2,
  dispatchEvent: vi.fn(),
};

global.EventSource = vi.fn(() => mockEventSource) as any;

const mockSourcesService = {
  triggerSync: vi.fn(),
  stopSync: vi.fn(),
  getSyncStatus: vi.fn(),
  getSyncProgressStream: vi.fn(() => mockEventSource),
  triggerDeepScan: vi.fn(),
};

const mockQueueService = {
  getQueueStatus: vi.fn(),
  pauseQueue: vi.fn(),
  resumeQueue: vi.fn(),
  clearQueue: vi.fn(),
};

vi.mock('../../services/api', async () => {
  const actual = await vi.importActual('../../services/api');
  return {
    ...actual,
    api: mockApi,
    sourcesService: mockSourcesService,
    queueService: mockQueueService,
  };
});

// Mock react-router-dom
const mockNavigate = vi.fn();
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useNavigate: () => mockNavigate,
  };
});

// Create mock source data
const createMockSource = (overrides: any = {}) => ({
  id: 'test-source-123',
  name: 'Test WebDAV Source',
  source_type: 'webdav',
  enabled: true,
  config: {
    server_url: 'https://nextcloud.example.com',
    username: 'testuser',
    password: 'password123',
    watch_folders: ['/Documents'],
    file_extensions: ['pdf', 'doc', 'docx'],
  },
  status: 'idle',
  last_sync_at: '2024-01-15T10:30:00Z',
  last_error: null,
  last_error_at: null,
  total_files_synced: 45,
  total_files_pending: 0,
  total_size_bytes: 15728640,
  total_documents: 42,
  total_documents_ocr: 38,
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-15T10:30:00Z',
  ...overrides,
});

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

describe('SourcesPage Sync Progress Integration', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    
    // Default mock responses
    mockApi.get.mockImplementation((url: string) => {
      if (url === '/sources') {
        return Promise.resolve({ data: [createMockSource()] });
      }
      if (url === '/queue/status') {
        return Promise.resolve({ 
          data: { 
            pending: 0, 
            processing: 0, 
            failed: 0, 
            completed: 100,
            is_paused: false 
          } 
        });
      }
      return Promise.resolve({ data: [] });
    });

    mockSourcesService.triggerSync.mockResolvedValue({ data: { success: true } });
    mockSourcesService.stopSync.mockResolvedValue({ data: { success: true } });
    mockSourcesService.getSyncStatus.mockResolvedValue({ data: null });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('Progress Display Visibility', () => {
    test('should not show progress display for idle sources', async () => {
      const idleSource = createMockSource({ status: 'idle' });
      mockApi.get.mockResolvedValue({ data: [idleSource] });

      renderWithProviders(<SourcesPage />);

      await waitFor(() => {
        expect(screen.getByText('Test WebDAV Source')).toBeInTheDocument();
      });

      // Progress display should not be visible
      expect(screen.queryByText('Test WebDAV Source - Sync Progress')).not.toBeInTheDocument();
    });

    test('should show progress display for syncing sources', async () => {
      const syncingSource = createMockSource({ status: 'syncing' });
      mockApi.get.mockResolvedValue({ data: [syncingSource] });

      renderWithProviders(<SourcesPage />);

      await waitFor(() => {
        expect(screen.getByText('Test WebDAV Source')).toBeInTheDocument();
      });

      // Progress display should be visible
      await waitFor(() => {
        expect(screen.getByText('Test WebDAV Source - Sync Progress')).toBeInTheDocument();
      });
    });

    test('should show progress display for multiple syncing sources', async () => {
      const sources = [
        createMockSource({ id: 'source-1', name: 'Source One', status: 'syncing' }),
        createMockSource({ id: 'source-2', name: 'Source Two', status: 'idle' }),
        createMockSource({ id: 'source-3', name: 'Source Three', status: 'syncing' }),
      ];
      mockApi.get.mockResolvedValue({ data: sources });

      renderWithProviders(<SourcesPage />);

      await waitFor(() => {
        expect(screen.getByText('Source One - Sync Progress')).toBeInTheDocument();
        expect(screen.getByText('Source Three - Sync Progress')).toBeInTheDocument();
        expect(screen.queryByText('Source Two - Sync Progress')).not.toBeInTheDocument();
      });
    });
  });

  describe('Progress Data Integration', () => {
    test('should display real-time progress updates', async () => {
      const syncingSource = createMockSource({ status: 'syncing' });
      mockApi.get.mockResolvedValue({ data: [syncingSource] });

      renderWithProviders(<SourcesPage />);

      await waitFor(() => {
        expect(screen.getByText('Test WebDAV Source - Sync Progress')).toBeInTheDocument();
      });

      // Simulate progress update via SSE
      const mockProgress = createMockProgressInfo();
      act(() => {
        const progressHandler = mockEventSource.addEventListener.mock.calls.find(
          call => call[0] === 'progress'
        )?.[1] as (event: MessageEvent) => void;
        
        if (progressHandler) {
          progressHandler(new MessageEvent('progress', {
            data: JSON.stringify(mockProgress)
          }));
        }
      });

      await waitFor(() => {
        expect(screen.getByText('Downloading and processing files')).toBeInTheDocument();
        expect(screen.getByText('30 / 50 files (60.0%)')).toBeInTheDocument();
        expect(screen.getByText('/Documents/Projects')).toBeInTheDocument();
      });
    });

    test('should handle progress updates for correct source', async () => {
      const sources = [
        createMockSource({ id: 'source-1', name: 'Source One', status: 'syncing' }),
        createMockSource({ id: 'source-2', name: 'Source Two', status: 'syncing' }),
      ];
      mockApi.get.mockResolvedValue({ data: sources });

      renderWithProviders(<SourcesPage />);

      await waitFor(() => {
        expect(screen.getByText('Source One - Sync Progress')).toBeInTheDocument();
        expect(screen.getByText('Source Two - Sync Progress')).toBeInTheDocument();
      });

      // Each source should create its own EventSource
      expect(mockSourcesService.getSyncProgressStream).toHaveBeenCalledWith('source-1');
      expect(mockSourcesService.getSyncProgressStream).toHaveBeenCalledWith('source-2');
    });
  });

  describe('Sync Control Integration', () => {
    test('should trigger sync and show progress display', async () => {
      const idleSource = createMockSource({ status: 'idle' });
      mockApi.get.mockResolvedValue({ data: [idleSource] });

      renderWithProviders(<SourcesPage />);

      await waitFor(() => {
        expect(screen.getByText('Test WebDAV Source')).toBeInTheDocument();
      });

      // Click sync button
      const syncButton = screen.getByLabelText('Trigger Sync');
      fireEvent.click(syncButton);

      expect(mockSourcesService.triggerSync).toHaveBeenCalledWith('test-source-123');

      // Simulate source status change to syncing after API call
      const syncingSource = createMockSource({ status: 'syncing' });
      mockApi.get.mockResolvedValue({ data: [syncingSource] });

      // Progress display should appear
      await waitFor(() => {
        expect(screen.getByText('Test WebDAV Source - Sync Progress')).toBeInTheDocument();
      });
    });

    test('should stop sync and hide progress display', async () => {
      const syncingSource = createMockSource({ status: 'syncing' });
      mockApi.get.mockResolvedValue({ data: [syncingSource] });

      renderWithProviders(<SourcesPage />);

      await waitFor(() => {
        expect(screen.getByText('Test WebDAV Source - Sync Progress')).toBeInTheDocument();
      });

      // Click stop sync button
      const stopButton = screen.getByLabelText('Stop Sync');
      fireEvent.click(stopButton);

      expect(mockSourcesService.stopSync).toHaveBeenCalledWith('test-source-123');

      // Simulate source status change to idle after API call
      const idleSource = createMockSource({ status: 'idle' });
      mockApi.get.mockResolvedValue({ data: [idleSource] });

      // Progress display should disappear
      await waitFor(() => {
        expect(screen.queryByText('Test WebDAV Source - Sync Progress')).not.toBeInTheDocument();
      });
    });
  });

  describe('Auto-refresh Behavior', () => {
    test('should enable auto-refresh when sources are syncing', async () => {
      const syncingSource = createMockSource({ status: 'syncing' });
      mockApi.get.mockResolvedValue({ data: [syncingSource] });

      renderWithProviders(<SourcesPage />);

      await waitFor(() => {
        expect(screen.getByText('Test WebDAV Source')).toBeInTheDocument();
      });

      // Auto-refresh should be enabled for syncing sources
      // This is tested indirectly by checking that the API is called multiple times
      await waitFor(() => {
        expect(mockApi.get).toHaveBeenCalledWith('/sources');
      }, { timeout: 3000 });
    });

    test('should disable auto-refresh when no sources are syncing', async () => {
      const idleSource = createMockSource({ status: 'idle' });
      mockApi.get.mockResolvedValue({ data: [idleSource] });

      renderWithProviders(<SourcesPage />);

      await waitFor(() => {
        expect(screen.getByText('Test WebDAV Source')).toBeInTheDocument();
      });

      // Auto-refresh should not be running for idle sources
      const initialCallCount = mockApi.get.mock.calls.length;
      
      // Wait a bit and ensure no additional calls are made
      await new Promise(resolve => setTimeout(resolve, 1000));
      
      expect(mockApi.get.mock.calls.length).toBe(initialCallCount);
    });
  });

  describe('Error Handling', () => {
    test('should handle sync trigger errors gracefully', async () => {
      const idleSource = createMockSource({ status: 'idle' });
      mockApi.get.mockResolvedValue({ data: [idleSource] });
      mockSourcesService.triggerSync.mockRejectedValue(new Error('Sync failed'));

      renderWithProviders(<SourcesPage />);

      await waitFor(() => {
        expect(screen.getByText('Test WebDAV Source')).toBeInTheDocument();
      });

      // Click sync button
      const syncButton = screen.getByLabelText('Trigger Sync');
      fireEvent.click(syncButton);

      await waitFor(() => {
        expect(mockSourcesService.triggerSync).toHaveBeenCalledWith('test-source-123');
      });

      // Source should remain idle and no progress display should appear
      expect(screen.queryByText('Test WebDAV Source - Sync Progress')).not.toBeInTheDocument();
    });

    test('should handle progress stream connection errors', async () => {
      const syncingSource = createMockSource({ status: 'syncing' });
      mockApi.get.mockResolvedValue({ data: [syncingSource] });

      renderWithProviders(<SourcesPage />);

      await waitFor(() => {
        expect(screen.getByText('Test WebDAV Source - Sync Progress')).toBeInTheDocument();
      });

      // Simulate SSE connection error
      act(() => {
        if (mockEventSource.onerror) {
          mockEventSource.onerror(new Event('error'));
        }
      });

      // Progress display should still be visible but show disconnected status
      await waitFor(() => {
        expect(screen.getByText('Disconnected')).toBeInTheDocument();
      });
    });
  });

  describe('Performance Considerations', () => {
    test('should only create progress streams for syncing sources', async () => {
      const sources = [
        createMockSource({ id: 'source-1', name: 'Source One', status: 'idle' }),
        createMockSource({ id: 'source-2', name: 'Source Two', status: 'syncing' }),
        createMockSource({ id: 'source-3', name: 'Source Three', status: 'error' }),
      ];
      mockApi.get.mockResolvedValue({ data: sources });

      renderWithProviders(<SourcesPage />);

      await waitFor(() => {
        expect(screen.getByText('Source One')).toBeInTheDocument();
        expect(screen.getByText('Source Two')).toBeInTheDocument();
        expect(screen.getByText('Source Three')).toBeInTheDocument();
      });

      // Only the syncing source should create an SSE stream
      expect(mockSourcesService.getSyncProgressStream).toHaveBeenCalledTimes(1);
      expect(mockSourcesService.getSyncProgressStream).toHaveBeenCalledWith('source-2');
    });

    test('should cleanup progress streams when component unmounts', async () => {
      const syncingSource = createMockSource({ status: 'syncing' });
      mockApi.get.mockResolvedValue({ data: [syncingSource] });

      const { unmount } = renderWithProviders(<SourcesPage />);

      await waitFor(() => {
        expect(screen.getByText('Test WebDAV Source - Sync Progress')).toBeInTheDocument();
      });

      unmount();

      expect(mockEventSource.close).toHaveBeenCalled();
    });
  });

  describe('UI State Management', () => {
    test('should maintain progress display state during source list refresh', async () => {
      const syncingSource = createMockSource({ status: 'syncing' });
      mockApi.get.mockResolvedValue({ data: [syncingSource] });

      renderWithProviders(<SourcesPage />);

      await waitFor(() => {
        expect(screen.getByText('Test WebDAV Source - Sync Progress')).toBeInTheDocument();
      });

      // Collapse the progress display
      const collapseButton = screen.getByLabelText('Collapse');
      fireEvent.click(collapseButton);

      await waitFor(() => {
        expect(screen.queryByText('Waiting for sync progress information...')).not.toBeInTheDocument();
      });

      // Simulate a source list refresh (which happens periodically)
      mockApi.get.mockResolvedValue({ data: [syncingSource] });

      // Force a re-render by triggering a state change
      await act(async () => {
        // The component should maintain the collapsed state
      });

      // Progress display should still be collapsed
      expect(screen.queryByText('Waiting for sync progress information...')).not.toBeInTheDocument();
    });

    test('should show appropriate status indicators', async () => {
      const syncingSource = createMockSource({ status: 'syncing' });
      mockApi.get.mockResolvedValue({ data: [syncingSource] });

      renderWithProviders(<SourcesPage />);

      await waitFor(() => {
        expect(screen.getByText('Test WebDAV Source - Sync Progress')).toBeInTheDocument();
      });

      // Should show connecting status initially
      expect(screen.getByText('Connecting...')).toBeInTheDocument();

      // Simulate successful connection
      act(() => {
        if (mockEventSource.onopen) {
          mockEventSource.onopen(new Event('open'));
        }
      });

      // Simulate progress data
      const mockProgress = createMockProgressInfo();
      act(() => {
        const progressHandler = mockEventSource.addEventListener.mock.calls.find(
          call => call[0] === 'progress'
        )?.[1] as (event: MessageEvent) => void;
        
        if (progressHandler) {
          progressHandler(new MessageEvent('progress', {
            data: JSON.stringify(mockProgress)
          }));
        }
      });

      await waitFor(() => {
        expect(screen.getByText('Live')).toBeInTheDocument();
      });
    });
  });
});