import { RouterProvider } from 'react-aria-components';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it, vi } from 'vitest';
import { DocumentCard } from './DocumentCard';

const base = { title: 'Lease.pdf', thumbnail: <span data-testid="thumb" /> };

describe('DocumentCard', () => {
  it('opens via a named button', async () => {
    const onOpen = vi.fn();
    render(
      <ul>
        <DocumentCard {...base} onOpen={onOpen} />
      </ul>,
    );
    await userEvent.click(screen.getByRole('button', { name: 'Lease.pdf' }));
    expect(onOpen).toHaveBeenCalled();
  });

  it('renders a link when given href', () => {
    render(
      <ul>
        <DocumentCard {...base} href="/documents/1" />
      </ul>,
    );
    expect(screen.getByRole('link', { name: 'Lease.pdf' })).toHaveAttribute('href', '/documents/1');
  });

  it('hands routerOptions to the router with its link', async () => {
    const navigate = vi.fn();
    render(
      <RouterProvider navigate={navigate}>
        <ul>
          <DocumentCard {...base} href="/home?document=1" routerOptions={{ state: { drawers: { document: true } } }} />
        </ul>
      </RouterProvider>,
    );
    await userEvent.setup().click(screen.getByRole('link', { name: 'Lease.pdf' }));
    expect(navigate).toHaveBeenCalledWith('/home?document=1', { state: { drawers: { document: true } } });
  });

  it('selects with a named checkbox and marks state', async () => {
    const onSel = vi.fn();
    const { container } = render(
      <ul>
        <DocumentCard {...base} onOpen={() => {}} selectLabel="Select Lease.pdf" isSelected onSelectionChange={onSel} isChanged />
      </ul>,
    );
    expect(container.querySelector('li')).toHaveAttribute('data-selected', 'true');
    expect(container.querySelector('li')).toHaveAttribute('data-changed', 'true');
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Lease.pdf' }));
    expect(onSel).toHaveBeenCalledWith(false);
  });

  it('shows meta, status, labels, source and quick actions', () => {
    render(
      <ul>
        <DocumentCard
          {...base}
          onOpen={() => {}}
          meta="PDF · 3 pp"
          status={<span>Indexed</span>}
          labels={<span>Taxes</span>}
          source={<span>Nextcloud</span>}
          quickActions={<button type="button">Download</button>}
        />
      </ul>,
    );
    for (const text of ['PDF · 3 pp', 'Indexed', 'Taxes', 'Nextcloud']) expect(screen.getByText(text)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Download' })).toBeInTheDocument();
  });

  it('keeps a very long title in the accessible name while clamping visually', () => {
    const long = `${'Quarterly_statement_'.repeat(6)}final.pdf`;
    render(
      <ul>
        <DocumentCard {...base} title={long} onOpen={() => {}} />
      </ul>,
    );
    expect(screen.getByRole('button', { name: long })).toBeInTheDocument();
    expect(readFileSync(resolve(__dirname, 'DocumentCard.module.css'), 'utf8')).toMatch(/\.name\s*\{[^}]*-webkit-line-clamp:\s*2/);
  });
});

it('puts flags in the details, never over the preview, and rings the whole card when changed', () => {
  const { container } = render(<ul><DocumentCard {...base} onOpen={() => {}} isChanged flags={<span>New</span>} status={<span>Indexed</span>} /></ul>);
  const thumb = container.querySelector('[data-thumb]') as HTMLElement;
  expect(thumb).not.toBeNull();
  expect(within(thumb).queryByText('New')).toBeNull();
  expect(screen.getByText('New').closest('[data-details]')).not.toBeNull();
  const css = readFileSync(resolve(__dirname, 'DocumentCard.module.css'), 'utf8');
  expect(css).not.toMatch(/\.flags\s*\{[^}]*position:\s*absolute/);
  expect(css).not.toMatch(/\[data-changed\]\s*\.thumb/);
  expect(css).toMatch(/\.card\[data-changed\]\s*\{[^}]*--new-fill/);
});

it('reports the open when it is a link too', async () => {
  const onOpen = vi.fn();
  render(<RouterProvider navigate={() => undefined}><ul><DocumentCard {...base} href="/documents/1" onOpen={onOpen} /></ul></RouterProvider>);
  await userEvent.click(screen.getByRole('link', { name: 'Lease.pdf' }));
  expect(onOpen).toHaveBeenCalled();
});
