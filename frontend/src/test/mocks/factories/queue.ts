/**
 * Queue and OCR factory for generating realistic test data
 */

import { faker } from '@faker-js/faker'
import { 
  QueueStats, 
  OcrResponse,
  OcrRetryStatsResponse,
  OcrRetryFailureReason,
  SyncProgressInfo,
  MockSyncProgress,
  FactoryOptions 
} from '../api/types'

faker.seed(12345)

/**
 * Create mock queue statistics
 */
export const createMockQueueStats = (overrides: Partial<QueueStats> = {}): QueueStats => {
  const pendingCount = faker.number.int({ min: 0, max: 100 })
  const processingCount = faker.number.int({ min: 0, max: 5 })
  const failedCount = faker.number.int({ min: 0, max: 20 })
  
  return {
    pending_count: pendingCount,
    processing_count: processingCount,
    completed_count: faker.number.int({ min: 100, max: 1000 }),
    failed_count: failedCount,
    total_processed: faker.number.int({ min: 500, max: 5000 }),
    completed_today: faker.number.int({ min: 0, max: 200 }),
    avg_wait_time_minutes: pendingCount > 0 ? faker.number.int({ min: 1, max: 30 }) : undefined,
    oldest_pending_minutes: pendingCount > 0 ? faker.number.int({ min: 5, max: 120 }) : undefined,
    ...overrides,
  }
}

/**
 * Create mock OCR response
 */
export const createMockOcrResponse = (overrides: Partial<OcrResponse> = {}): OcrResponse => {
  const hasOcrText = faker.datatype.boolean(0.8)
  const ocrStatus = hasOcrText ? 'completed' : faker.helpers.arrayElement(['failed', 'pending', 'processing'])
  
  return {
    document_id: faker.string.uuid(),
    filename: `${faker.system.fileName()}.pdf`,
    has_ocr_text: hasOcrText,
    ocr_text: hasOcrText ? faker.lorem.paragraphs(5, '\n\n') : undefined,
    ocr_confidence: hasOcrText ? faker.number.float({ min: 0.1, max: 1.0, fractionDigits: 3 }) : undefined,
    ocr_word_count: hasOcrText ? faker.number.int({ min: 50, max: 2000 }) : undefined,
    ocr_processing_time_ms: hasOcrText ? faker.number.int({ min: 500, max: 15000 }) : undefined,
    ocr_status: ocrStatus,
    ocr_error: ocrStatus === 'failed' ? faker.lorem.sentence() : undefined,
    ocr_completed_at: hasOcrText ? faker.date.recent().toISOString() : undefined,
    ...overrides,
  }
}

/**
 * Create mock OCR retry statistics
 */
export const createMockRetryStats = (overrides: Partial<OcrRetryStatsResponse> = {}): OcrRetryStatsResponse => ({
  failure_reasons: [
    {
      reason: 'Low image quality',
      count: faker.number.int({ min: 5, max: 50 }),
      avg_file_size_mb: faker.number.float({ min: 0.5, max: 25.0, fractionDigits: 2 }),
      first_occurrence: faker.date.past({ years: 1 }).toISOString(),
      last_occurrence: faker.date.recent().toISOString(),
    },
    {
      reason: 'Unsupported language',
      count: faker.number.int({ min: 2, max: 20 }),
      avg_file_size_mb: faker.number.float({ min: 1.0, max: 15.0, fractionDigits: 2 }),
      first_occurrence: faker.date.past({ years: 1 }).toISOString(),
      last_occurrence: faker.date.recent().toISOString(),
    },
    {
      reason: 'File corruption',
      count: faker.number.int({ min: 1, max: 10 }),
      avg_file_size_mb: faker.number.float({ min: 0.1, max: 5.0, fractionDigits: 2 }),
      first_occurrence: faker.date.past({ years: 1 }).toISOString(),
      last_occurrence: faker.date.recent().toISOString(),
    },
  ],
  file_types: [
    {
      mime_type: 'application/pdf',
      count: faker.number.int({ min: 20, max: 100 }),
      avg_file_size_mb: faker.number.float({ min: 2.0, max: 50.0, fractionDigits: 2 }),
    },
    {
      mime_type: 'image/jpeg',
      count: faker.number.int({ min: 10, max: 50 }),
      avg_file_size_mb: faker.number.float({ min: 0.5, max: 10.0, fractionDigits: 2 }),
    },
    {
      mime_type: 'image/png',
      count: faker.number.int({ min: 5, max: 30 }),
      avg_file_size_mb: faker.number.float({ min: 0.3, max: 8.0, fractionDigits: 2 }),
    },
  ],
  total_failed: faker.number.int({ min: 10, max: 200 }),
  ...overrides,
})

/**
 * Create mock sync progress information
 */
export const createMockSyncProgress = (overrides: Partial<MockSyncProgress> = {}): MockSyncProgress => {
  const filesFound = faker.number.int({ min: 10, max: 1000 })
  const filesProcessed = faker.number.int({ min: 0, max: filesFound })
  const directoriesFound = faker.number.int({ min: 1, max: 50 })
  const directoriesProcessed = faker.number.int({ min: 0, max: directoriesFound })
  const elapsedTimeSecs = faker.number.int({ min: 10, max: 3600 })
  const processingRate = filesProcessed > 0 ? filesProcessed / elapsedTimeSecs : 0
  const progressPercent = filesFound > 0 ? (filesProcessed / filesFound) * 100 : 0
  const remainingFiles = filesFound - filesProcessed
  const estimatedTimeRemaining = processingRate > 0 && remainingFiles > 0 ? 
    remainingFiles / processingRate : undefined

  return {
    source_id: faker.string.uuid(),
    phase: faker.helpers.arrayElement(['discovery', 'processing', 'cleanup', 'completed']),
    phase_description: faker.lorem.sentence(),
    elapsed_time_secs: elapsedTimeSecs,
    directories_found: directoriesFound,
    directories_processed: directoriesProcessed,
    files_found: filesFound,
    files_processed: filesProcessed,
    bytes_processed: faker.number.int({ min: 1024 * 1024, max: 1024 * 1024 * 1024 }), // 1MB to 1GB
    processing_rate_files_per_sec: processingRate,
    files_progress_percent: progressPercent,
    estimated_time_remaining_secs: estimatedTimeRemaining,
    current_directory: faker.system.directoryPath(),
    current_file: faker.datatype.boolean(0.7) ? faker.system.fileName() : undefined,
    errors: faker.number.int({ min: 0, max: 5 }),
    warnings: faker.number.int({ min: 0, max: 10 }),
    is_active: faker.datatype.boolean(0.8),
    ...overrides,
  }
}

/**
 * Create sync progress with specific characteristics
 */
export const createMockSyncProgressWithScenario = (scenario: string): MockSyncProgress => {
  const baseProgress = createMockSyncProgress()

  switch (scenario) {
    case 'just_started':
      return {
        ...baseProgress,
        phase: 'discovery',
        phase_description: 'Starting directory scan...',
        elapsed_time_secs: faker.number.int({ min: 1, max: 30 }),
        files_processed: 0,
        directories_processed: 0,
        files_progress_percent: 0,
        estimated_time_remaining_secs: undefined,
        current_directory: baseProgress.current_directory,
        current_file: undefined,
        is_active: true,
      }

    case 'in_progress':
      const filesFound = faker.number.int({ min: 100, max: 500 })
      const filesProcessed = faker.number.int({ min: 20, max: filesFound * 0.8 })
      return {
        ...baseProgress,
        phase: 'processing',
        phase_description: 'Processing documents...',
        files_found: filesFound,
        files_processed: filesProcessed,
        files_progress_percent: (filesProcessed / filesFound) * 100,
        processing_rate_files_per_sec: faker.number.float({ min: 0.5, max: 5.0, fractionDigits: 2 }),
        estimated_time_remaining_secs: faker.number.int({ min: 60, max: 1800 }),
        current_file: faker.system.fileName(),
        is_active: true,
      }

    case 'nearly_complete':
      const totalFiles = faker.number.int({ min: 50, max: 200 })
      const processedFiles = Math.floor(totalFiles * 0.95)
      return {
        ...baseProgress,
        phase: 'cleanup',
        phase_description: 'Finalizing sync...',
        files_found: totalFiles,
        files_processed: processedFiles,
        files_progress_percent: 95,
        estimated_time_remaining_secs: faker.number.int({ min: 5, max: 60 }),
        is_active: true,
      }

    case 'completed':
      const completedFiles = faker.number.int({ min: 50, max: 300 })
      return {
        ...baseProgress,
        phase: 'completed',
        phase_description: 'Sync completed successfully',
        files_found: completedFiles,
        files_processed: completedFiles,
        files_progress_percent: 100,
        estimated_time_remaining_secs: 0,
        current_file: undefined,
        is_active: false,
      }

    case 'with_errors':
      return {
        ...baseProgress,
        errors: faker.number.int({ min: 5, max: 20 }),
        warnings: faker.number.int({ min: 10, max: 30 }),
        phase_description: `Processing with ${baseProgress.errors} errors`,
      }

    case 'idle':
      return {
        ...baseProgress,
        phase: 'completed',
        phase_description: 'No active sync',
        is_active: false,
        current_file: undefined,
        estimated_time_remaining_secs: undefined,
      }

    default:
      console.warn(`Unknown sync progress scenario: ${scenario}`)
      return baseProgress
  }
}

/**
 * Create queue data for specific test scenarios
 */
export const createQueueScenarios = (): Record<string, any> => ({
  empty_queue: createMockQueueStats({ 
    pending_count: 0, 
    processing_count: 0, 
    failed_count: 0 
  }),
  busy_queue: createMockQueueStats({ 
    pending_count: 45, 
    processing_count: 3, 
    failed_count: 12 
  }),
  idle_queue: createMockQueueStats({ 
    pending_count: 0, 
    processing_count: 0, 
    failed_count: 0,
    completed_today: 150,
  }),
  backlogged_queue: createMockQueueStats({ 
    pending_count: 200, 
    processing_count: 5, 
    failed_count: 50,
    avg_wait_time_minutes: 45,
    oldest_pending_minutes: 120,
  }),
  sync_progress: {
    just_started: createMockSyncProgressWithScenario('just_started'),
    in_progress: createMockSyncProgressWithScenario('in_progress'),
    nearly_complete: createMockSyncProgressWithScenario('nearly_complete'),
    completed: createMockSyncProgressWithScenario('completed'),
    with_errors: createMockSyncProgressWithScenario('with_errors'),
    idle: createMockSyncProgressWithScenario('idle'),
  },
})

/**
 * Reset faker seed for consistent test results
 */
export const resetQueueFactorySeed = (seed: number = 12345) => {
  faker.seed(seed)
}