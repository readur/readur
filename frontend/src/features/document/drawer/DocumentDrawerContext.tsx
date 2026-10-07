import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from 'react';
import { useDrawerParam, type DrawerParam } from '../../../lib/useDrawerParam';
import { DOCUMENTS_CHANGED_EVENT } from '../../shell/useDocumentTotal';
import type { LabelData } from '../../labels';

/** The URL param naming the document open in the drawer, on any page. */
export const DOCUMENT_PARAM = 'document';

/** What the drawer changed on a document, for a list to update its row without refetching. */
export interface DocumentPatch {
  labels?: LabelData[];
  ocr_status?: string | null;
}

/**
 * The documents the current page shows, registered so the drawer can step through them (↑/↓),
 * title a document before it loads, and tell the page about changes.
 */
export interface DocumentListProvider {
  ids: readonly string[];
  peek?: (id: string) => { name: string } | null;
  onChanged?: (id: string, patch: DocumentPatch) => void;
  onDeleted?: (id: string) => void;
}

type Register = (provider: DocumentListProvider | null, previous?: DocumentListProvider) => void;

const RegisterContext = createContext<Register | null>(null);
const ListContext = createContext<DocumentListProvider | null>(null);

/**
 * Holds the current page's document list for the drawer. Pages only see the stable register
 * function, so a list change re-renders the drawer, not the page.
 */
export function DocumentDrawerProvider({ children }: { children: ReactNode }) {
  const [list, setList] = useState<DocumentListProvider | null>(null);
  const register = useCallback<Register>((provider, previous) => {
    if (provider) setList(provider);
    else setList((current) => (current === previous ? null : current));
  }, []);
  return (
    <RegisterContext.Provider value={register}>
      <ListContext.Provider value={list}>{children}</ListContext.Provider>
    </RegisterContext.Provider>
  );
}

/** Registers the page's documents while it is mounted. Memoise `provider`; the last one wins. */
export function useRegisterDocumentList(provider: DocumentListProvider | null) {
  const register = useContext(RegisterContext);
  useEffect(() => {
    if (!register || !provider) return undefined;
    register(provider);
    return () => register(null, provider);
  }, [register, provider]);
}

/** The list the current page registered, if any. */
export function useDocumentList(): DocumentListProvider | null {
  return useContext(ListContext);
}

export interface DocumentDrawer extends DrawerParam {
  /** Closes the drawer if it shows one of `ids` (they were deleted, say). */
  closeIfOpen: (ids: readonly string[]) => void;
}

/** Opens and closes the document drawer through `?document=` on the current page. */
export function useDocumentDrawer(): DocumentDrawer {
  const drawer = useDrawerParam(DOCUMENT_PARAM);
  const { id, close } = drawer;
  const closeIfOpen = useCallback(
    (ids: readonly string[]) => {
      if (id !== null && ids.includes(id)) close();
    },
    [id, close],
  );
  return { ...drawer, closeIfOpen };
}

/** Tells every list on screen that documents changed (deleted from the drawer, say). */
export function notifyDocumentsChanged(detail: { deleted?: string[] } = {}) {
  window.dispatchEvent(new CustomEvent(DOCUMENTS_CHANGED_EVENT, { detail }));
}
