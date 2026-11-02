import type { ReactNode } from 'react';
import { render } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { vi } from 'vitest';
import { AuthContext } from '../../../contexts/AuthContext';
import type { Document, OcrResponse } from '../../../services/api';
import { ToastProvider } from '../../../ui';
import type { UserRole } from '../../../types/generated';
import { DocumentDrawerProvider, useRegisterDocumentList, type DocumentListProvider } from '../drawer/DocumentDrawerContext';
import { DocumentDrawerHost } from '../drawer/DocumentDrawerHost';

export const makeDocument = (overrides: Partial<Document> = {}): Document => ({
  id: 'doc-1',
  filename: 'invoice.pdf',
  original_filename: 'invoice.pdf',
  file_path: '/uploads/invoice.pdf',
  file_size: 2_097_152,
  mime_type: 'application/pdf',
  tags: [],
  labels: [],
  created_at: '2025-06-15T10:00:00Z',
  updated_at: '2025-06-15T10:05:00Z',
  user_id: 'user-1',
  username: 'ada',
  has_ocr_text: true,
  ocr_status: 'completed',
  ocr_confidence: 95.5,
  ocr_word_count: 290,
  ocr_processing_time_ms: 1500,
  ...overrides,
});

export const makeOcr = (overrides: Partial<OcrResponse> = {}): OcrResponse => ({
  id: 'doc-1',
  filename: 'invoice.pdf',
  has_ocr_text: true,
  ocr_text: 'Invoice 42\nTotal due: 120 EUR\nPay the invoice by Friday.',
  ocr_confidence: 95.5,
  ocr_word_count: 290,
  ocr_processing_time_ms: 1500,
  ocr_status: 'completed',
  detected_language: null,
  pages_processed: null,
  ...overrides,
});

export const testUser = { id: 'user-1', username: 'ada', email: 'ada@example.com', role: 'user' as UserRole };

/** jsdom has no object URLs; give each blob a fake one. */
export function stubObjectUrls() {
  Object.defineProperty(URL, 'createObjectURL', { configurable: true, writable: true, value: vi.fn(() => 'blob:fake') });
  Object.defineProperty(URL, 'revokeObjectURL', { configurable: true, writable: true, value: vi.fn() });
}

export function stubClipboard() {
  const writeText = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText } });
  return writeText;
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

export function Providers({ children, user = testUser }: { children: ReactNode; user?: typeof testUser | null }) {
  const auth = { user, loading: false, login: vi.fn(), register: vi.fn(), logout: vi.fn() };
  return (
    <ToastProvider>
      <AuthContext.Provider value={auth}>{children}</AuthContext.Provider>
    </ToastProvider>
  );
}

function ListPage({ list }: { list?: DocumentListProvider }) {
  useRegisterDocumentList(list ?? null);
  return <LocationProbe />;
}

export interface RenderDrawerOptions {
  /** The page URL with the drawer open on it. */
  path?: string;
  /** What the page under the drawer registers (its rows, for ↑/↓ and row updates). */
  list?: DocumentListProvider;
  user?: typeof testUser | null;
}

/** The document drawer over a stand-in page at `path` (default `/documents?document=doc-1`). */
export function renderDrawer({ path = '/documents?document=doc-1', list, user = testUser }: RenderDrawerOptions = {}) {
  return render(
    <Providers user={user}>
      <MemoryRouter initialEntries={[path]}>
        <DocumentDrawerProvider>
          <Routes>
            <Route path="*" element={<ListPage list={list} />} />
          </Routes>
          <DocumentDrawerHost />
        </DocumentDrawerProvider>
      </MemoryRouter>
    </Providers>,
  );
}
