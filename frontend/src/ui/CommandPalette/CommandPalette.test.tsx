import { useState } from 'react';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Button } from '../Button';
import { CommandPalette } from './CommandPalette';
import type { CommandItem, CommandSource } from './types';
import { useCommandPaletteShortcut } from './useCommandPaletteShortcut';

function makeSources(onSelect: (id: string) => void) {
  const docs: CommandItem[] = [
    { id: 'd1', title: 'Invoice March', subtitle: 'invoice-03.pdf', onSelect: () => onSelect('d1') },
    { id: 'd2', title: 'Invoice April', onSelect: () => onSelect('d2') },
  ];
  const searchDocs = vi.fn(async (q: string) => docs.filter((d) => d.title.toLowerCase().includes(q.toLowerCase())));
  const searchLabels = vi.fn(async (q: string) =>
    q.startsWith('inv') ? [{ id: 'l1', title: 'Invoices', onSelect: () => onSelect('l1') }] : [],
  );
  const sources: CommandSource[] = [
    { id: 'documents', label: 'Documents', search: searchDocs },
    { id: 'labels', label: 'Labels', search: searchLabels },
  ];
  return { sources, searchDocs, searchLabels };
}

function Harness({ sources, withShortcut }: { sources: CommandSource[]; withShortcut?: boolean }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      {withShortcut ? <Shortcut onOpen={() => setOpen(true)} /> : null}
      <Button onPress={() => setOpen(true)}>Open palette</Button>
      <input aria-label="Notes" />
      <CommandPalette isOpen={open} onOpenChange={setOpen} sources={sources} />
    </>
  );
}

function Shortcut({ onOpen }: { onOpen: () => void }) {
  useCommandPaletteShortcut(onOpen);
  return null;
}

async function openPalette(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: 'Open palette' }));
  return screen.findByRole('dialog', { name: 'Command palette' });
}

describe('CommandPalette', () => {
  it('opens a named dialog with a focused search field', async () => {
    const user = userEvent.setup();
    render(<Harness sources={makeSources(() => {}).sources} />);
    await openPalette(user);
    await waitFor(() => expect(screen.getByRole('searchbox', { name: 'Search' })).toHaveFocus());
    expect(screen.getByRole('menu', { name: 'Results' })).toBeInTheDocument();
  });

  it('debounces typing into a single search per source', async () => {
    const user = userEvent.setup();
    const { sources, searchDocs } = makeSources(() => {});
    render(<Harness sources={sources} />);
    await openPalette(user);
    await user.keyboard('inv');
    await waitFor(() => expect(searchDocs).toHaveBeenCalledWith('inv'));
    const queries = searchDocs.mock.calls.map(([q]) => q);
    expect(queries).not.toContain('i');
    expect(queries).not.toContain('in');
    expect(queries.filter((q) => q === 'inv')).toHaveLength(1);
  });

  it('does not query sources with an empty string unless they opt in', async () => {
    const user = userEvent.setup();
    const { sources, searchDocs } = makeSources(() => {});
    const searchCommands = vi.fn(async () => [{ id: 'c1', title: 'Go to settings', onSelect: () => {} }]);
    render(<Harness sources={[...sources, { id: 'commands', label: 'Commands', search: searchCommands, searchesEmpty: true }]} />);
    await openPalette(user);
    expect(await screen.findByRole('menuitem', { name: 'Go to settings' })).toBeInTheDocument();
    expect(searchCommands).toHaveBeenCalledWith('');
    expect(searchDocs).not.toHaveBeenCalled();
  });

  it('shows a hint and runs no search on open when no source opts in', async () => {
    const user = userEvent.setup();
    const { sources, searchDocs, searchLabels } = makeSources(() => {});
    render(<Harness sources={sources} />);
    await openPalette(user);
    expect(await screen.findByText('Type to search')).toBeInTheDocument();
    await new Promise((r) => setTimeout(r, 200));
    expect(searchDocs).not.toHaveBeenCalled();
    expect(searchLabels).not.toHaveBeenCalled();
  });

  it('does not select anything when Enter is pressed before the search resolves', async () => {
    const onSelect = vi.fn();
    const user = userEvent.setup();
    render(<Harness sources={makeSources(onSelect).sources} />);
    await openPalette(user);
    await user.keyboard('invoice{Enter}');
    expect(onSelect).not.toHaveBeenCalled();
    expect(screen.getByRole('dialog', { name: 'Command palette' })).toBeInTheDocument();
  });

  it('hides results from an earlier query while the new one is searched', async () => {
    const onSelect = vi.fn();
    const user = userEvent.setup();
    render(<Harness sources={makeSources(onSelect).sources} />);
    await openPalette(user);
    await user.keyboard('invoice');
    await screen.findByRole('menuitem', { name: 'Invoice March' });
    await user.keyboard('x');
    expect(screen.queryByRole('menuitem', { name: 'Invoice March' })).not.toBeInTheDocument();
    expect(screen.getByText('Searching…', { selector: 'p' })).toBeInTheDocument();
    await user.keyboard('{Enter}');
    expect(onSelect).not.toHaveBeenCalled();
    // Once the new query resolves, only its results are shown.
    expect(await screen.findByRole('menuitem', { name: 'Invoices' })).toBeInTheDocument();
    expect(screen.queryByRole('menuitem', { name: 'Invoice March' })).not.toBeInTheDocument();
  });

  it('groups results under source headings', async () => {
    const user = userEvent.setup();
    render(<Harness sources={makeSources(() => {}).sources} />);
    await openPalette(user);
    await user.keyboard('inv');
    expect(await screen.findByRole('menuitem', { name: 'Invoice March' })).toBeInTheDocument();
    const groups = screen.getAllByRole('group');
    expect(groups.map((g) => g.getAttribute('aria-labelledby') && document.getElementById(g.getAttribute('aria-labelledby')!)?.textContent)).toEqual([
      'Documents',
      'Labels',
    ]);
    expect(screen.getByRole('menuitem', { name: 'Invoice March' })).toHaveAccessibleDescription('invoice-03.pdf');
  });

  it('selects with arrow keys and Enter, then closes', async () => {
    const onSelect = vi.fn();
    const user = userEvent.setup();
    render(<Harness sources={makeSources(onSelect).sources} />);
    await openPalette(user);
    await user.keyboard('invoice');
    await screen.findByRole('menuitem', { name: 'Invoice April' });
    // The first result is focused automatically; one ArrowDown moves to the second.
    await user.keyboard('{ArrowDown}{Enter}');
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(onSelect).toHaveBeenCalledWith('d2');
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });

  it('shows "No results" when nothing matches', async () => {
    const user = userEvent.setup();
    render(<Harness sources={makeSources(() => {}).sources} />);
    await openPalette(user);
    await user.keyboard('zzz');
    expect(await screen.findByText('No results')).toBeInTheDocument();
  });

  it('shows a loading state while sources are pending', async () => {
    const user = userEvent.setup();
    const pending: CommandSource[] = [{ id: 'slow', label: 'Slow', search: () => new Promise<CommandItem[]>(() => {}) }];
    render(<Harness sources={pending} />);
    await openPalette(user);
    await user.keyboard('a');
    expect(await screen.findByRole('status')).toHaveTextContent('Searching…');
  });

  it('closes on Escape even with text in the field', async () => {
    const user = userEvent.setup();
    render(<Harness sources={makeSources(() => {}).sources} />);
    const trigger = screen.getByRole('button', { name: 'Open palette' });
    await openPalette(user);
    await user.keyboard('abc{Escape}');
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    await waitFor(() => expect(trigger).toHaveFocus());
  });
});

describe('useCommandPaletteShortcut', () => {
  it('opens with Meta+K and Ctrl+K', async () => {
    const user = userEvent.setup();
    render(<Harness withShortcut sources={makeSources(() => {}).sources} />);
    await user.keyboard('{Meta>}k{/Meta}');
    expect(await screen.findByRole('dialog', { name: 'Command palette' })).toBeInTheDocument();
    await user.keyboard('{Escape}');
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    await user.keyboard('{Control>}k{/Control}');
    expect(await screen.findByRole('dialog', { name: 'Command palette' })).toBeInTheDocument();
  });

  it('opens with / outside text fields only', async () => {
    const user = userEvent.setup();
    render(<Harness withShortcut sources={makeSources(() => {}).sources} />);
    await user.click(screen.getByRole('textbox', { name: 'Notes' }));
    await user.keyboard('/');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'Notes' })).toHaveValue('/');
    act(() => screen.getByRole('button', { name: 'Open palette' }).focus());
    await user.keyboard('/');
    expect(await screen.findByRole('dialog', { name: 'Command palette' })).toBeInTheDocument();
  });
});
