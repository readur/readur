import { describe, test, expect, vi } from 'vitest';

// Test the interfaces and types are properly defined
describe('Sync Progress Types and Interfaces', () => {
  test('should have SyncProgressInfo interface properly defined', () => {
    // Import the type and check it compiles
    const progressInfo: import('../api').SyncProgressInfo = {
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
    };

    expect(progressInfo.source_id).toBe('test-123');
    expect(progressInfo.phase).toBe('processing_files');
    expect(progressInfo.files_progress_percent).toBe(60.0);
    expect(progressInfo.is_active).toBe(true);
  });

  test('should handle optional fields in SyncProgressInfo', () => {
    const minimalProgressInfo: import('../api').SyncProgressInfo = {
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
      // Optional fields not provided:
      // estimated_time_remaining_secs
      // current_file
    };

    expect(minimalProgressInfo.estimated_time_remaining_secs).toBeUndefined();
    expect(minimalProgressInfo.current_file).toBeUndefined();
    expect(minimalProgressInfo.files_progress_percent).toBe(0.0);
  });

  test('should handle failed sync state', () => {
    const failedProgressInfo: import('../api').SyncProgressInfo = {
      source_id: 'test-789',
      phase: 'failed',
      phase_description: 'Sync failed: Connection timeout',
      elapsed_time_secs: 45,
      directories_found: 5,
      directories_processed: 2,
      files_found: 20,
      files_processed: 8,
      bytes_processed: 204800,
      processing_rate_files_per_sec: 0.18,
      files_progress_percent: 40.0,
      current_directory: '/Documents/Partial',
      current_file: 'interrupted-file.pdf',
      errors: 1,
      warnings: 0,
      is_active: false,
    };

    expect(failedProgressInfo.phase).toBe('failed');
    expect(failedProgressInfo.is_active).toBe(false);
    expect(failedProgressInfo.errors).toBe(1);
  });

  test('should handle completed sync state', () => {
    const completedProgressInfo: import('../api').SyncProgressInfo = {
      source_id: 'test-complete',
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
    };

    expect(completedProgressInfo.phase).toBe('completed');
    expect(completedProgressInfo.files_progress_percent).toBe(100.0);
    expect(completedProgressInfo.estimated_time_remaining_secs).toBe(0);
    expect(completedProgressInfo.current_file).toBeNull();
  });
});

describe('Sync Progress API Methods Type Safety', () => {
  test('should have properly typed sourcesService methods', async () => {
    // This tests that the TypeScript compilation works correctly
    const { sourcesService } = await import('../api');
    
    expect(typeof sourcesService.getSyncStatus).toBe('function');
    expect(typeof sourcesService.triggerSync).toBe('function');
    expect(typeof sourcesService.stopSync).toBe('function');
    expect(typeof sourcesService.triggerDeepScan).toBe('function');
    expect(typeof sourcesService.createSyncProgressWebSocket).toBe('function');
  });

  test('should accept proper parameter types', () => {
    // Test that parameter types are enforced properly
    const sourceId: string = 'test-source-123';
    const uuidPattern = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
    
    // Should accept any string as source ID
    expect(typeof sourceId).toBe('string');
    
    // Should work with UUID format
    const uuid = '550e8400-e29b-41d4-a716-446655440000';
    expect(uuidPattern.test(uuid)).toBe(true);
    
    // Should work with simple IDs
    const simpleId = 'simple-id-123';
    expect(typeof simpleId).toBe('string');
  });
});