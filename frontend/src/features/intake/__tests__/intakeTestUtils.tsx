import type { ReactNode } from 'react';
import { act, render } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import i18n from 'i18next';
import { I18nextProvider, initReactI18next } from 'react-i18next';
import { vi } from 'vitest';
import { AuthContext } from '../../../contexts/AuthContext';
import { FeatureFlagsContext } from '../../../contexts/FeatureFlagsContext';
import { NotificationProvider } from '../../../contexts/NotificationContext';
import { ToastProvider } from '../../../ui';
import { acknowledgeAll, flushLit } from '../../board/litStore';

// No resources: every string renders its English default, with interpolation.
if (!i18n.isInitialized) {
  void i18n.use(initReactI18next).init({
    lng: 'en',
    fallbackLng: 'en',
    resources: {},
    interpolation: { escapeValue: false },
    react: { useSuspense: false },
  });
}

class MemoryStorage implements Storage {
  private data = new Map<string, string>();
  get length() {
    return this.data.size;
  }
  clear() {
    this.data.clear();
  }
  getItem(key: string) {
    return this.data.has(key) ? (this.data.get(key) as string) : null;
  }
  key(i: number) {
    return Array.from(this.data.keys())[i] ?? null;
  }
  removeItem(key: string) {
    this.data.delete(key);
  }
  setItem(key: string, value: string) {
    this.data.set(key, String(value));
  }
}

/** Lets pending requests resolve and their state updates land inside act(). */
export async function settle(): Promise<void> {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
}

/** Fresh storage and an empty change-tracking store. */
export function resetIntakeState(): void {
  Object.defineProperty(window, 'localStorage', { value: new MemoryStorage(), configurable: true, writable: true });
  acknowledgeAll();
  flushLit();
}

export function LocationProbe() {
  const { pathname, search } = useLocation();
  return (
    <output aria-label="location">
      {pathname}
      {search}
    </output>
  );
}

export interface RenderIntakeOptions {
  path?: string;
  role?: 'Admin' | 'User';
  perUserWatch?: boolean;
  signedIn?: boolean;
}

export function renderIntake(ui: ReactNode, { path = '/intake', role = 'Admin', perUserWatch = false, signedIn = true }: RenderIntakeOptions = {}) {
  const auth = {
    user: signedIn ? { id: '1', username: 'ada', email: 'ada@example.com', role } : null,
    loading: false,
    login: vi.fn(),
    register: vi.fn(),
    logout: vi.fn(),
  };
  const flags = {
    flags: { allowLocalAuth: true, oidcEnabled: false, enablePerUserWatch: perUserWatch },
    loading: false,
    error: null,
  };
  return render(
    <I18nextProvider i18n={i18n}>
      <MemoryRouter initialEntries={[path]}>
        <AuthContext.Provider value={auth as never}>
          <FeatureFlagsContext.Provider value={flags}>
            <NotificationProvider>
              <ToastProvider>
                <Routes>
                  <Route
                    path="*"
                    element={
                      <>
                        {ui}
                        <LocationProbe />
                      </>
                    }
                  />
                </Routes>
              </ToastProvider>
            </NotificationProvider>
          </FeatureFlagsContext.Provider>
        </AuthContext.Provider>
      </MemoryRouter>
    </I18nextProvider>,
  );
}

/** Minimal source row for the connections board. */
export function source(id: string, overrides: Record<string, unknown> = {}) {
  return {
    id,
    name: `Source ${id}`,
    source_type: 'webdav',
    enabled: true,
    config: {
      server_url: 'https://cloud.example.com',
      username: 'ada',
      password: 'secret',
      server_type: 'nextcloud',
      watch_folders: ['/Documents'],
      file_extensions: ['pdf'],
      auto_sync: true,
      sync_interval_minutes: 60,
    },
    status: 'idle',
    last_sync_at: new Date(Date.now() - 5 * 60_000).toISOString(),
    last_error: null,
    last_error_at: null,
    total_files_synced: 12,
    total_files_pending: 0,
    total_size_bytes: 2048,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-02T00:00:00Z',
    total_documents: 12,
    total_documents_ocr: 10,
    validation_status: null,
    last_validation_at: null,
    validation_score: null,
    validation_issues: null,
    ...overrides,
  };
}

/** Minimal row of GET /documents/failed. */
export function failedDoc(id: string, overrides: Record<string, unknown> = {}) {
  return {
    id,
    filename: `${id}.pdf`,
    original_filename: `${id}.pdf`,
    file_path: null,
    file_size: 1024,
    mime_type: 'application/pdf',
    content: null,
    tags: [],
    ocr_text: null,
    ocr_confidence: null,
    ocr_word_count: null,
    ocr_processing_time_ms: null,
    failure_reason: 'ocr_timeout',
    failure_stage: 'ocr',
    existing_document_id: null,
    ingestion_source: 'upload',
    error_message: 'Tesseract timed out',
    retry_count: 1,
    last_retry_at: null,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-02T00:00:00Z',
    failure_category: 'Timeout',
    ...overrides,
  };
}

export function failedList(docs: unknown[]) {
  return {
    documents: docs,
    pagination: { total: docs.length, limit: 25, offset: 0, total_pages: 1 },
    statistics: { total_failed: docs.length, by_stage: {}, by_reason: {} },
  };
}

/** Row of GET /documents/failed/ocr: a real document whose OCR failed. */
export function ocrDoc(id: string, overrides: Record<string, unknown> = {}) {
  return {
    id,
    filename: `${id}.pdf`,
    original_filename: `${id}.pdf`,
    file_size: 1024,
    mime_type: 'application/pdf',
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-02T00:00:00Z',
    tags: [],
    ocr_status: 'failed',
    ocr_error: 'Tesseract timed out',
    ocr_failure_reason: 'ocr_timeout',
    ocr_completed_at: null,
    retry_count: 1,
    last_attempt_at: null,
    can_retry: true,
    failure_category: 'Timeout',
    ...overrides,
  };
}

export function ocrList(docs: unknown[]) {
  return {
    documents: docs,
    pagination: { total: docs.length, limit: 25, offset: 0, has_more: false },
    statistics: { total_failed: docs.length, failure_categories: [] },
  };
}
