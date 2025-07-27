import { describe, test, expect, vi, beforeEach } from 'vitest';
import { sourcesService } from '../api';

// Mock axios
const mockApi = {
  get: vi.fn(),
  post: vi.fn(),
};

vi.mock('../api', async () => {
  const actual = await vi.importActual('../api');
  return {
    ...actual,
    api: mockApi,
    sourcesService: {
      ...actual.sourcesService,
      getSyncStatus: vi.fn(),
      getSyncProgressStream: vi.fn(),
    },
  };
});

// Define EventSource constants
const EVENTSOURCE_CONNECTING = 0;
const EVENTSOURCE_OPEN = 1;
const EVENTSOURCE_CLOSED = 2;

// Mock EventSource
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

global.EventSource = vi.fn(() => mockEventSource) as any;
(global.EventSource as any).CONNECTING = EVENTSOURCE_CONNECTING;
(global.EventSource as any).OPEN = EVENTSOURCE_OPEN;
(global.EventSource as any).CLOSED = EVENTSOURCE_CLOSED;

describe('API Sync Progress Services', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('getSyncStatus', () => {
    test('should call correct endpoint for sync status', async () => {
      const mockResponse = {
        data: {
          source_id: 'test-123',
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
        }
      };

      mockApi.get.mockResolvedValue(mockResponse);

      const result = await sourcesService.getSyncStatus('test-123');

      expect(mockApi.get).toHaveBeenCalledWith('/sources/test-123/sync/status');
      expect(result).toBe(mockResponse);
    });

    test('should handle empty response for inactive sync', async () => {
      const mockResponse = { data: null };
      mockApi.get.mockResolvedValue(mockResponse);

      const result = await sourcesService.getSyncStatus('test-456');

      expect(mockApi.get).toHaveBeenCalledWith('/sources/test-456/sync/status');
      expect(result).toBe(mockResponse);
    });

    test('should handle API errors gracefully', async () => {
      const mockError = new Error('Network error');
      mockApi.get.mockRejectedValue(mockError);

      await expect(sourcesService.getSyncStatus('test-789')).rejects.toThrow('Network error');
      expect(mockApi.get).toHaveBeenCalledWith('/sources/test-789/sync/status');
    });

    test('should handle different source IDs correctly', async () => {
      const sourceIds = ['uuid-1', 'uuid-2', 'special-chars-123!@#'];
      
      for (const sourceId of sourceIds) {
        mockApi.get.mockResolvedValue({ data: null });
        
        await sourcesService.getSyncStatus(sourceId);
        
        expect(mockApi.get).toHaveBeenCalledWith(`/sources/${sourceId}/sync/status`);
      }
    });
  });

  describe('getSyncProgressStream', () => {
    test('should create EventSource with correct URL', () => {
      const sourceId = 'test-source-123';
      
      const eventSource = sourcesService.getSyncProgressStream(sourceId);

      expect(global.EventSource).toHaveBeenCalledWith(`/api/sources/${sourceId}/sync/progress`);
      expect(eventSource).toBe(mockEventSource);
    });

    test('should handle different source IDs in stream URL', () => {
      const testCases = [
        'simple-id',
        'uuid-with-dashes-123-456-789',
        'special_chars_id',
      ];

      testCases.forEach(sourceId => {
        vi.clearAllMocks();
        
        sourcesService.getSyncProgressStream(sourceId);
        
        expect(global.EventSource).toHaveBeenCalledWith(`/api/sources/${sourceId}/sync/progress`);
      });
    });

    test('should return new EventSource instance each time', () => {
      const sourceId = 'test-123';
      
      const stream1 = sourcesService.getSyncProgressStream(sourceId);
      const stream2 = sourcesService.getSyncProgressStream(sourceId);
      
      expect(global.EventSource).toHaveBeenCalledTimes(2);
      expect(stream1).toBe(mockEventSource);
      expect(stream2).toBe(mockEventSource);
    });
  });

  describe('API Integration with existing methods', () => {
    test('should maintain compatibility with existing sync methods', async () => {
      // Test that new methods don't interfere with existing ones
      mockApi.post.mockResolvedValue({ data: { success: true } });
      
      await sourcesService.triggerSync('test-123');
      expect(mockApi.post).toHaveBeenCalledWith('/sources/test-123/sync');
      
      await sourcesService.stopSync('test-123');
      expect(mockApi.post).toHaveBeenCalledWith('/sources/test-123/sync/stop');
      
      await sourcesService.triggerDeepScan('test-123');
      expect(mockApi.post).toHaveBeenCalledWith('/sources/test-123/deep-scan');
    });

    test('should have all expected methods in sourcesService', () => {
      const expectedMethods = [
        'triggerSync',
        'triggerDeepScan', 
        'stopSync',
        'getSyncStatus',
        'getSyncProgressStream',
      ];

      expectedMethods.forEach(method => {
        expect(sourcesService).toHaveProperty(method);
        expect(typeof sourcesService[method]).toBe('function');
      });
    });
  });

  describe('Error Scenarios', () => {
    test('should handle network failures for sync status', async () => {
      const networkError = {
        response: {
          status: 500,
          data: { error: 'Internal server error' }
        }
      };
      
      mockApi.get.mockRejectedValue(networkError);
      
      await expect(sourcesService.getSyncStatus('test-123')).rejects.toEqual(networkError);
    });

    test('should handle 404 for non-existent source', async () => {
      const notFoundError = {
        response: {
          status: 404,
          data: { error: 'Source not found' }
        }
      };
      
      mockApi.get.mockRejectedValue(notFoundError);
      
      await expect(sourcesService.getSyncStatus('non-existent')).rejects.toEqual(notFoundError);
    });

    test('should handle 401 unauthorized errors', async () => {
      const unauthorizedError = {
        response: {
          status: 401,
          data: { error: 'Unauthorized' }
        }
      };
      
      mockApi.get.mockRejectedValue(unauthorizedError);
      
      await expect(sourcesService.getSyncStatus('test-123')).rejects.toEqual(unauthorizedError);
    });
  });

  describe('Response Data Validation', () => {
    test('should handle complete progress response correctly', async () => {
      const completeResponse = {
        data: {
          source_id: 'test-123',
          phase: 'completed',
          phase_description: 'Sync completed successfully',
          elapsed_time_secs: 300,
          directories_found: 25,
          directories_processed: 25,
          files_found: 150,
          files_processed: 150,
          bytes_processed: 15728640, // 15 MB
          processing_rate_files_per_sec: 0.5,
          files_progress_percent: 100.0,
          estimated_time_remaining_secs: 0,
          current_directory: '/Documents/Final',
          current_file: null,
          errors: 0,
          warnings: 2,
          is_active: false,
        }
      };

      mockApi.get.mockResolvedValue(completeResponse);
      
      const result = await sourcesService.getSyncStatus('test-123');
      
      expect(result.data.phase).toBe('completed');
      expect(result.data.files_progress_percent).toBe(100.0);
      expect(result.data.is_active).toBe(false);
      expect(result.data.current_file).toBeNull();
    });

    test('should handle minimal progress response', async () => {
      const minimalResponse = {
        data: {
          source_id: 'test-456',
          phase: 'initializing',
          phase_description: 'Initializing sync operation',
          elapsed_time_secs: 5,
          directories_found: 0,
          directories_processed: 0,
          files_found: 0,
          files_processed: 0,
          bytes_processed: 0,
          processing_rate_files_per_sec: 0.0,
          files_progress_percent: 0.0,
          current_directory: '',
          errors: 0,
          warnings: 0,
          is_active: true,
        }
      };

      mockApi.get.mockResolvedValue(minimalResponse);
      
      const result = await sourcesService.getSyncStatus('test-456');
      
      expect(result.data.phase).toBe('initializing');
      expect(result.data.files_progress_percent).toBe(0.0);
      expect(result.data.is_active).toBe(true);
    });

    test('should handle failed sync response', async () => {
      const failedResponse = {
        data: {
          source_id: 'test-789',
          phase: 'failed',
          phase_description: 'Sync failed: Connection timeout',
          elapsed_time_secs: 45,
          directories_found: 5,
          directories_processed: 2,
          files_found: 20,
          files_processed: 8,
          bytes_processed: 204800, // 200 KB
          processing_rate_files_per_sec: 0.18,
          files_progress_percent: 40.0,
          current_directory: '/Documents/Partial',
          current_file: 'interrupted-file.pdf',
          errors: 1,
          warnings: 0,
          is_active: false,
        }
      };

      mockApi.get.mockResolvedValue(failedResponse);
      
      const result = await sourcesService.getSyncStatus('test-789');
      
      expect(result.data.phase).toBe('failed');
      expect(result.data.phase_description).toContain('Connection timeout');
      expect(result.data.errors).toBe(1);
      expect(result.data.is_active).toBe(false);
    });
  });
});