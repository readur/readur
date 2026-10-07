import { useMemo } from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import { DOCUMENTS_CHANGED_EVENT } from '../../../shell/useDocumentTotal';
import {
  DocumentDrawerProvider,
  notifyDocumentsChanged,
  useDocumentDrawer,
  useDocumentList,
  useRegisterDocumentList,
} from '../DocumentDrawerContext';

function Page({ ids }: { ids: string[] }) {
  const list = useMemo(() => ({ ids }), [ids]);
  useRegisterDocumentList(list);
  return null;
}

function Drawer() {
  const list = useDocumentList();
  const drawer = useDocumentDrawer();
  return (
    <>
      <output aria-label="list">{list ? list.ids.join(',') : 'none'}</output>
      <output aria-label="open">{drawer.id ?? 'none'}</output>
      <button type="button" onClick={() => drawer.open('d2')}>open d2</button>
      <button type="button" onClick={() => drawer.closeIfOpen(['d9'])}>close if d9</button>
      <button type="button" onClick={() => drawer.closeIfOpen(['d2', 'd3'])}>close if d2</button>
    </>
  );
}

function Url() {
  const { pathname, search } = useLocation();
  return <output aria-label="url">{pathname + search}</output>;
}

const out = (name: string) => screen.getByRole('status', { name });

describe('document drawer context', () => {
  it('shares the list the current page registers, and forgets it when the page goes', () => {
    const { rerender } = render(
      <MemoryRouter>
        <DocumentDrawerProvider>
          <Page ids={['d1', 'd2']} />
          <Drawer />
        </DocumentDrawerProvider>
      </MemoryRouter>,
    );
    expect(out('list')).toHaveTextContent('d1,d2');
    rerender(
      <MemoryRouter>
        <DocumentDrawerProvider>
          <Drawer />
        </DocumentDrawerProvider>
      </MemoryRouter>,
    );
    expect(out('list')).toHaveTextContent('none');
  });

  it('uses the latest registration when two pages register', () => {
    render(
      <MemoryRouter>
        <DocumentDrawerProvider>
          <Page ids={['a']} />
          <Page ids={['b']} />
          <Drawer />
        </DocumentDrawerProvider>
      </MemoryRouter>,
    );
    expect(out('list')).toHaveTextContent('b');
  });

  it('opens through the URL and closes only when the open document is among the given ids', async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter initialEntries={['/documents?labels=work']}>
        <DocumentDrawerProvider>
          <Drawer />
          <Url />
        </DocumentDrawerProvider>
      </MemoryRouter>,
    );
    await user.click(screen.getByRole('button', { name: 'open d2' }));
    expect(out('url')).toHaveTextContent('/documents?labels=work&document=d2');
    await user.click(screen.getByRole('button', { name: 'close if d9' }));
    expect(out('open')).toHaveTextContent('d2');
    await user.click(screen.getByRole('button', { name: 'close if d2' }));
    expect(out('url')).toHaveTextContent('/documents?labels=work');
  });

  it('announces changed documents to every list on screen', () => {
    const listener = vi.fn();
    window.addEventListener(DOCUMENTS_CHANGED_EVENT, listener);
    notifyDocumentsChanged({ deleted: ['d1'] });
    window.removeEventListener(DOCUMENTS_CHANGED_EVENT, listener);
    expect(listener).toHaveBeenCalledTimes(1);
    expect((listener.mock.calls[0][0] as CustomEvent).detail).toEqual({ deleted: ['d1'] });
  });
});
