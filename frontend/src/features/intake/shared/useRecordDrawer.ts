import { useEffect } from 'react';
import { useDrawerParam } from '../../../lib/useDrawerParam';

/**
 * A drawer over one record of a loaded list, kept in the URL as `?<key>=<id>`. These records
 * exist only in the list, so once it has loaded, an id that is not on it is dropped.
 */
export function useRecordDrawer<T extends { id: string }>(key: string, rows: readonly T[], isLoaded: boolean) {
  const drawer = useDrawerParam(key);
  const { id, close } = drawer;
  const record = id ? (rows.find((r) => r.id === id) ?? null) : null;
  useEffect(() => {
    if (id && isLoaded && !rows.some((r) => r.id === id)) close();
  }, [id, isLoaded, rows, close]);
  return { record, open: drawer.open, close };
}
