import type { ReactNode } from 'react';
import { render } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import i18n from 'i18next';
import { I18nextProvider, initReactI18next } from 'react-i18next';
import { AuthContext } from '../../../contexts/AuthContext';
import { acknowledgeAll, flushLit } from '../litStore';
import { resetDocumentBaseline } from '../litFeeders';

// No resources: every string shows its English default, with interpolation.
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

/** Clean slate for the module-level stores and the storage they persist to. */
export function resetBoardState(): void {
  Object.defineProperty(window, 'localStorage', { value: new MemoryStorage(), configurable: true, writable: true });
  acknowledgeAll();
  flushLit();
  resetDocumentBaseline();
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

export function renderPage(page: ReactNode, role: 'Admin' | 'User' = 'Admin') {
  const auth = {
    user: { id: '1', username: 'ada', email: 'ada@example.com', role },
    loading: false,
    login: async () => {},
    register: async () => {},
    logout: () => {},
  };
  return render(
    <I18nextProvider i18n={i18n}>
      <MemoryRouter initialEntries={['/board']}>
        <AuthContext.Provider value={auth as never}>
          <Routes>
            <Route
              path="*"
              element={
                <>
                  {page}
                  <LocationProbe />
                </>
              }
            />
          </Routes>
        </AuthContext.Provider>
      </MemoryRouter>
    </I18nextProvider>,
  );
}

export const doc = (id: string, over: Record<string, unknown> = {}) => ({
  id,
  filename: `${id}.pdf`,
  original_filename: `${id}.pdf`,
  file_size: 2048,
  mime_type: 'application/pdf',
  created_at: new Date(Date.now() - 5 * 60_000).toISOString(),
  has_ocr_text: true,
  ocr_status: 'completed',
  ...over,
});
