import type { ReactNode } from 'react';
import { act, render } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { I18nextProvider } from 'react-i18next';
import i18n from 'i18next';
import '../../../test/test-utils';
import { ToastProvider } from '../../../ui';
import Library from '../Library';
import SearchRoute from '../SearchRoute';
import { documentService, searchService } from './serviceMocks';

function DocumentStandIn() {
  return <p>Document page</p>;
}

/** The Search page at `url` (default `/search`). */
export const renderSearch = (url = '/search', extra?: ReactNode) => renderLibrary(url, extra);

function LocationProbe() {
  const { pathname, search } = useLocation();
  return (
    <output aria-label="location">
      {pathname}
      {search}
    </output>
  );
}

export function currentUrl(): string {
  return document.querySelector('output[aria-label="location"]')?.textContent ?? '';
}

/** The Library and Search routes (plus stand-ins for the pages they link to) at `url`. */
export function renderLibrary(url = '/documents', extra?: ReactNode) {
  return render(
    <I18nextProvider i18n={i18n}>
      <ToastProvider>
        <MemoryRouter initialEntries={[url]}>
          <Routes>
            <Route path="/documents" element={<Library />} />
            <Route path="/search" element={<SearchRoute />} />
            <Route path="/documents/:id" element={<DocumentStandIn />} />
            <Route path="/intake" element={<p>Intake page</p>} />
          </Routes>
          <LocationProbe />
          {extra}
        </MemoryRouter>
      </ToastProvider>
    </I18nextProvider>,
  );
}

/** The query string the last list call was made with, as the API sees it. */
export const lastListParams = () => documentService.listFiltered.mock.calls.at(-1)?.[0];
export const lastSearchParams = () => searchService.enhancedSearch.mock.calls.at(-1)?.[0];
export const lastTimelineParams = () => searchService.getTimeline.mock.calls.at(-1)?.[0];

/** Switch the Library to the thumbnail grid before rendering. */
export const useGrid = () => window.localStorage.setItem('readur.library.view', 'grid');

/** Let pending requests resolve so their state updates land inside act. */
export async function settle() {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
}
