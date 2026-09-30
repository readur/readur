/**
 * Service mocks shared by the Intake tests. Test files wire them in with
 *   vi.mock('../../../services/api', async () => (await import('./intakeMocks')).apiModule)
 * (path adjusted), so the test and the component see the same vi.fn instances.
 * The global setup resets every mock before each test; call `serveDefaults()` in beforeEach.
 */
import { vi } from 'vitest';
import { ErrorCodes, ErrorHelper } from '../../../services/errors';

const fn = () => vi.fn();

export const api = { get: fn(), post: fn(), put: fn(), delete: fn() };

export const sourcesService = {
  list: fn(),
  create: fn(),
  update: fn(),
  remove: fn(),
  testConnection: fn(),
  validate: fn(),
  estimate: fn(),
  estimateWithConfig: fn(),
  triggerSync: fn(),
  triggerDeepScan: fn(),
  stopSync: fn(),
  getSyncStatus: fn(),
  createSyncProgressWebSocket: fn(),
};

export const queueService = {
  getStats: fn(),
  requeueFailed: fn(),
  getOcrStatus: fn(),
  pauseOcr: fn(),
  resumeOcr: fn(),
};

export const ocrService = {
  getAvailableLanguages: fn(),
  getHealthStatus: fn(),
  retryWithLanguage: fn(),
  listFailedDocuments: fn(),
  viewFailedDocument: fn(),
};

export const documentService = {
  retryOcr: fn(),
  bulkRetryOcr: fn(),
  bulkDelete: fn(),
  getRetryRecommendations: fn(),
  getDocumentRetryHistory: fn(),
  getDuplicates: fn(),
  deleteLowConfidence: fn(),
  deleteFailedOcr: fn(),
  downloadFile: fn(),
};

export const sourceErrorService = { listFailures: fn() };

export const userWatchService = {
  getUserWatchDirectory: fn(),
  createUserWatchDirectory: fn(),
  deleteUserWatchDirectory: fn(),
};

export const ignoredFilesService = { list: fn(), stats: fn(), remove: fn(), bulkRemove: fn() };

export const apiModule = {
  default: api,
  api,
  ErrorHelper,
  ErrorCodes,
  sourcesService,
  queueService,
  ocrService,
  documentService,
  sourceErrorService,
  userWatchService,
};

export const ignoredFilesModule = { ignoredFilesService };

export const ok = <T,>(data: T) => Promise.resolve({ data });

/** An axios-shaped error with an API error code. */
export function apiError(status: number, code?: string, message?: string) {
  return Object.assign(new Error(message ?? 'Request failed'), {
    isAxiosError: true,
    response: { status, data: { code, error: message ?? code, message } },
  });
}

export const LANGUAGES = {
  available_languages: [
    { code: 'eng', name: 'English', installed: true },
    { code: 'deu', name: 'German', installed: true },
  ],
  current_user_language: 'eng',
};

/** Harmless defaults: empty lists, successful actions. */
export function serveDefaults(): void {
  api.get.mockImplementation((url: string) => {
    if (url.startsWith('/labels')) return ok([]);
    return ok({});
  });
  api.post.mockImplementation(() => ok({ id: 'doc-new' }));
  sourcesService.list.mockImplementation(() => ok([]));
  sourcesService.create.mockImplementation(() => ok({ id: 'new' }));
  sourcesService.update.mockImplementation(() => ok({}));
  sourcesService.remove.mockImplementation(() => ok({}));
  sourcesService.testConnection.mockImplementation(() => ok({ success: true, message: 'Connected' }));
  sourcesService.validate.mockImplementation(() => ok({ success: true, message: 'Started' }));
  sourcesService.estimate.mockImplementation(() =>
    ok({ folders: [], total_files: 0, total_supported_files: 0, total_estimated_time_hours: 0, total_size_mb: 0 }),
  );
  sourcesService.triggerSync.mockImplementation(() => ok({}));
  sourcesService.triggerDeepScan.mockImplementation(() => ok({}));
  sourcesService.stopSync.mockImplementation(() => ok({}));
  // A progress socket that connects and then stays quiet.
  sourcesService.createSyncProgressWebSocket.mockImplementation(() => ({
    connect: () => Promise.resolve(),
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
    close: () => undefined,
  }));
  queueService.getStats.mockImplementation(() =>
    ok({ pending_count: 10, processing_count: 4, failed_count: 0, completed_today: 3 }),
  );
  queueService.requeueFailed.mockImplementation(() => ok({ requeued_count: 2 }));
  queueService.getOcrStatus.mockImplementation(() => ok({ is_paused: false, status: 'running' }));
  queueService.pauseOcr.mockImplementation(() => ok({ status: 'paused', message: '' }));
  queueService.resumeOcr.mockImplementation(() => ok({ status: 'resumed', message: '' }));
  ocrService.getAvailableLanguages.mockImplementation(() => ok(LANGUAGES));
  ocrService.retryWithLanguage.mockImplementation(() => ok({ success: true }));
  ocrService.listFailedDocuments.mockImplementation(() =>
    ok({ documents: [], pagination: { total: 0, limit: 25, offset: 0, total_pages: 0 }, statistics: { total_failed: 0, by_stage: {}, by_reason: {} } }),
  );
  ocrService.viewFailedDocument.mockImplementation(() => Promise.reject(apiError(404)));
  documentService.retryOcr.mockImplementation(() => ok({ success: true, estimated_wait_minutes: 2 }));
  documentService.bulkRetryOcr.mockImplementation(() =>
    ok({ success: true, message: 'ok', queued_count: 2, matched_count: 2, documents: [], estimated_total_time_minutes: 1 }),
  );
  documentService.bulkDelete.mockImplementation(() => ok({}));
  documentService.getRetryRecommendations.mockImplementation(() => ok({ recommendations: [], total_recommendations: 0 }));
  documentService.getDocumentRetryHistory.mockImplementation(() => ok({ document_id: 'x', retry_history: [], total_retries: 0 }));
  documentService.getDuplicates.mockImplementation(() =>
    ok({ duplicates: [], pagination: { total: 0, limit: 25, offset: 0, has_more: false }, statistics: { total_duplicate_groups: 0 } }),
  );
  documentService.deleteLowConfidence.mockImplementation(() => ok({ message: 'No documents', matched_count: 0, documents: [] }));
  documentService.deleteFailedOcr.mockImplementation(() => ok({ message: 'No documents', matched_count: 0, document_ids: [] }));
  documentService.downloadFile.mockImplementation(() => Promise.resolve());
  sourceErrorService.listFailures.mockImplementation(() => ok([]));
  userWatchService.getUserWatchDirectory.mockImplementation(() =>
    ok({ user_id: '1', username: 'ada', watch_directory_path: '/watch/ada', exists: true, enabled: true }),
  );
  userWatchService.createUserWatchDirectory.mockImplementation(() => ok({ success: true, message: 'Created' }));
  ignoredFilesService.list.mockImplementation(() => ok({ ignored_files: [], total: 0 }));
  ignoredFilesService.stats.mockImplementation(() =>
    ok({ total_ignored_files: 0, by_source_type: [], total_size_bytes: 0, most_recent_ignored_at: null }),
  );
  ignoredFilesService.remove.mockImplementation(() => ok({ message: 'Removed' }));
  ignoredFilesService.bulkRemove.mockImplementation(() => ok({ message: 'Removed' }));
}
