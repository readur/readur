import { beforeEach, describe, expect, it, vi } from 'vitest';
import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

vi.mock('../../services/api', () => {
  const api = { get: vi.fn(), post: vi.fn(), defaults: { headers: { common: {} as Record<string, string> } } };
  return { api, default: api, documentService: { list: vi.fn() } };
});

import { AuthProvider, useAuth } from '../AuthContext';
import { isLit, LIT_STORAGE_KEY, markLit } from '../../features/board/litStore';
import { BULK_ARRIVALS_KEY, useBulkArrivals } from '../../features/board/litFeeders';
import { readRecentSearches, RECENT_SEARCHES_KEY, saveRecentSearch } from '../../features/shell/recentSearches';

function Probe() {
  const { logout } = useAuth();
  const bulk = useBulkArrivals();
  return (
    <>
      <span data-testid="bulk">{bulk}</span>
      <button onClick={logout}>Log out</button>
    </>
  );
}

describe('AuthProvider logout', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('forgets per-browser activity state, and keeps preferences', async () => {
    localStorage.setItem('token', 'abc');
    localStorage.setItem(BULK_ARRIVALS_KEY, '40');
    localStorage.setItem('readur.board.dismissed.v1', '["a"]');
    localStorage.setItem('readur.board.lastSeen.v1', '123');
    localStorage.setItem('readur.intake.sourceFailures.v1', '["x"]');
    localStorage.setItem('readur.library.compact', 'true');
    localStorage.setItem('themeMode', 'dark');
    saveRecentSearch('tax 2024');
    markLit('document', 'd1', 'new');
    // Unflushed on purpose: the pending write must not bring the entry back after sign-out.
    markLit('source', 's1', 'failed');

    const user = userEvent.setup();
    render(
      <AuthProvider>
        <Probe />
      </AuthProvider>,
    );
    expect(screen.getByTestId('bulk')).toHaveTextContent('40');
    expect(isLit('document', 'd1')).toBe(true);

    await user.click(screen.getByRole('button', { name: 'Log out' }));
    await act(async () => {
      await Promise.resolve();
    });

    expect(localStorage.getItem('token')).toBeNull();
    expect(localStorage.getItem(LIT_STORAGE_KEY)).toBeNull();
    expect(localStorage.getItem(RECENT_SEARCHES_KEY)).toBeNull();
    expect(readRecentSearches()).toEqual([]);
    expect(localStorage.getItem(BULK_ARRIVALS_KEY)).toBeNull();
    expect(localStorage.getItem('readur.board.dismissed.v1')).toBeNull();
    expect(localStorage.getItem('readur.board.lastSeen.v1')).toBeNull();
    expect(localStorage.getItem('readur.intake.sourceFailures.v1')).toBeNull();
    expect(isLit('document', 'd1')).toBe(false);
    expect(isLit('source', 's1')).toBe(false);
    expect(screen.getByTestId('bulk')).toHaveTextContent('0');

    expect(localStorage.getItem('readur.library.compact')).toBe('true');
    expect(localStorage.getItem('themeMode')).toBe('dark');
  });
});
