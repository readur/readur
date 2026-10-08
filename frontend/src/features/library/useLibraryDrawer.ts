import { useMemo } from 'react';
import {
  useDocumentDrawer,
  useRegisterDocumentList,
  type DocumentDrawer,
  type DocumentListProvider,
} from '../document/drawer/DocumentDrawerContext';
import { displayName, type LibraryRow } from './data';

/**
 * Opens the page's rows in the document drawer and lends it the rows: ↑/↓ walks them, a row
 * titles the drawer before the document loads, and changes made there update the row.
 */
export function useLibraryDrawer(
  rows: readonly LibraryRow[],
  patchRow: (id: string, patch: Partial<LibraryRow>) => void,
  onDeleted: (id: string) => void,
): DocumentDrawer {
  const drawer = useDocumentDrawer();
  const list = useMemo<DocumentListProvider>(
    () => ({
      ids: rows.map((r) => r.id),
      peek: (id) => {
        const row = rows.find((r) => r.id === id);
        return row ? { name: displayName(row) } : null;
      },
      onChanged: patchRow,
      onDeleted,
    }),
    [rows, patchRow, onDeleted],
  );
  useRegisterDocumentList(list);
  return drawer;
}
