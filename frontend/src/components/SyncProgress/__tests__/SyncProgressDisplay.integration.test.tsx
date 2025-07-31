import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { screen, waitFor, act } from '@testing-library/react';
import { renderWithProviders } from '../../../test/test-utils';
import { SyncProgressDisplay } from '../SyncProgressDisplay';
import { MockSyncProgressManager } from '../../../services/syncProgress';
import { SyncProgressInfo } from '../../../services/api';

// Integration tests using MockSyncProgressManager
// These tests verify the component works with the actual hook and service layer

let mockManager: MockSyncProgressManager;

// Don't mock useSyncProgress - instead inject MockSyncProgressManager

const createMockProgressInfo = (overrides: Partial<SyncProgressInfo> = {}): SyncProgressInfo => ({
  source_id: 'test-source-123',
  phase: 'processing_files',
  phase_description: 'Processing files',
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
  current_file: 'document.pdf',
  errors: 0,
  warnings: 0,
  is_active: true,
  ...overrides,
});

describe('SyncProgressDisplay - Integration Tests', () => {
  beforeEach(() => {
    mockManager = new MockSyncProgressManager();
    vi.clearAllMocks();
  });

  afterEach(() => {
    mockManager.destroy();
  });

  test('should handle full sync progress lifecycle', async () => {
    renderWithProviders(
      <SyncProgressDisplay
        sourceId="test-source-123"
        sourceName="Test Source"
        isVisible={true}
        manager={mockManager}
      />
    );

    // Should start with connecting
    await waitFor(() => {
      expect(screen.getByText('Connecting...')).toBeInTheDocument();
    });

    // Should show connected
    await waitFor(() => {
      expect(screen.getByText('Connected')).toBeInTheDocument();
    });

    // Simulate progress update
    const mockProgress = createMockProgressInfo();
    await act(async () => {
      mockManager.simulateProgress(mockProgress);
    });

    // Should show live status and progress
    await waitFor(() => {
      expect(screen.getByText('Live')).toBeInTheDocument();
      expect(screen.getByText('Processing files')).toBeInTheDocument();
    });

    // Simulate heartbeat ending sync
    await act(async () => {
      mockManager.simulateHeartbeat({
        source_id: 'test-source-123',
        is_active: false,
        timestamp: Date.now(),
      });
    });

    // Should clear progress info
    await waitFor(() => {
      expect(screen.getByText('Waiting for sync progress information...')).toBeInTheDocument();
    });
  });

  test('should handle connection errors and recovery', async () => {
    renderWithProviders(
      <SyncProgressDisplay
        sourceId="test-source-123"
        sourceName="Test Source"
        isVisible={true}
        manager={mockManager}
      />
    );

    // Wait for connection
    await waitFor(() => {
      expect(screen.getByText('Connected')).toBeInTheDocument();
    });

    // Simulate connection failure
    await act(async () => {
      mockManager.simulateConnectionStatus('failed');
    });

    await waitFor(() => {
      expect(screen.getByText('Connection Failed')).toBeInTheDocument();
    });

    // Should show reconnect option
    expect(screen.getByRole('button', { name: /reconnect/i })).toBeInTheDocument();
  });

  test('should handle visibility changes', async () => {
    const { rerender } = renderWithProviders(
      <SyncProgressDisplay
        sourceId="test-source-123"
        sourceName="Test Source"
        isVisible={true}
        manager={mockManager}
      />
    );

    // Wait for connection
    await waitFor(() => {
      expect(screen.getByText('Connected')).toBeInTheDocument();
    });

    // Hide component
    rerender(
      <SyncProgressDisplay
        sourceId="test-source-123"
        sourceName="Test Source"
        isVisible={false}
        manager={mockManager}
      />
    );

    // Should not be visible
    expect(screen.queryByText('Test Source - Sync Progress')).not.toBeInTheDocument();

    // Show again
    rerender(
      <SyncProgressDisplay
        sourceId="test-source-123"
        sourceName="Test Source"
        isVisible={true}
        manager={mockManager}
      />
    );

    // Should reconnect
    await waitFor(() => {
      expect(screen.getByText('Connecting...')).toBeInTheDocument();
    });
  });
});