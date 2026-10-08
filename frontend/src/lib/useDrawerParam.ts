import { useCallback, useMemo, useRef } from 'react';
import { useLocation, useNavigate, type Location } from 'react-router-dom';

/** `location.state.drawers[key]` marks a history entry that `open()` pushed for that drawer. */
export const DRAWER_STATE = 'drawers';

type DrawerMarks = Record<string, true>;
type LocationState = Record<string, unknown> & { [DRAWER_STATE]?: DrawerMarks };

export interface DrawerParam {
  /** The record the URL names, or null when the drawer is closed. */
  id: string | null;
  /** Opens the drawer on `id`: a new history entry, or a replace when it is already open. */
  open(id: string): void;
  /** Moves the open drawer to another record (↑/↓) without adding history. */
  step(id: string): void;
  /** Closes the drawer: back to the entry before `open()`, or the same URL without the param. */
  close(): void;
  /** This page's URL with the drawer open on `id`, for real links (new tab, copy link). */
  href(id: string): string;
  /** History state to send with a link to `href(id)`, so `close()` can go back. */
  linkState: LocationState;
}

function stateOf(location: Location): LocationState {
  const { state } = location;
  return state && typeof state === 'object' ? (state as LocationState) : {};
}

function urlWith(location: Location, key: string, id: string | null): string {
  const params = new URLSearchParams(location.search);
  if (id === null) params.delete(key);
  else params.set(key, id);
  const search = params.toString();
  return `${location.pathname}${search ? `?${search}` : ''}${location.hash}`;
}

/**
 * Keeps one drawer in the URL as `?<key>=<id>`, next to whatever else the page puts there.
 * Opening adds a history entry so Back closes the drawer; moving between records replaces it.
 */
export function useDrawerParam(key: string): DrawerParam {
  const location = useLocation();
  const navigate = useNavigate();
  const id = new URLSearchParams(location.search).get(key);
  const state = stateOf(location);
  const opened = state[DRAWER_STATE]?.[key] === true;
  // The history entry already closed: a second close() before the URL changes (a delete handler
  // and a "that record is gone" effect both closing) must not go back a further entry.
  const closedEntry = useRef<string | null>(null);

  const linkState = useMemo<LocationState>(
    () => ({ ...state, [DRAWER_STATE]: { ...state[DRAWER_STATE], [key]: true } }),
    [state, key],
  );

  const open = useCallback(
    (next: string) => {
      if (id !== null) navigate(urlWith(location, key, next), { replace: true, state });
      else navigate(urlWith(location, key, next), { state: linkState });
    },
    [id, navigate, location, key, state, linkState],
  );

  const step = useCallback(
    (next: string) => navigate(urlWith(location, key, next), { replace: true, state }),
    [navigate, location, key, state],
  );

  const close = useCallback(() => {
    if (id === null || closedEntry.current === location.key) return;
    closedEntry.current = location.key;
    if (opened) {
      navigate(-1);
      return;
    }
    const { [key]: _drop, ...marks } = state[DRAWER_STATE] ?? {};
    navigate(urlWith(location, key, null), { replace: true, state: { ...state, [DRAWER_STATE]: marks } });
  }, [id, opened, navigate, location, key, state]);

  const href = useCallback((next: string) => urlWith(location, key, next), [location, key]);

  return { id, open, step, close, href, linkState };
}
