import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../services/api', async () => (await import('./intakeMocks')).apiModule);
vi.mock('../../../services/api/ignoredFiles', async () => (await import('./intakeMocks')).ignoredFilesModule);

import { IgnoredSection } from '../ignored/IgnoredSection';
import { formatBytes } from '../shared/format';
import { ignoredFilesService, ok, serveDefaults, sourcesService } from './intakeMocks';
import { renderIntake, resetIntakeState, source } from './intakeTestUtils';
import { createResponsiveMatchMediaMock } from '../../../test/pwa-test-utils';

function file(id: string, overrides: Record<string, unknown> = {}) {
  return {
    id,
    file_hash: `h${id}`,
    filename: `${id}.pdf`,
    original_filename: `${id}.pdf`,
    file_path: `/remote/${id}.pdf`,
    file_size: 1048576,
    mime_type: 'application/pdf',
    source_type: 'webdav',
    source_path: `/Documents/${id}.pdf`,
    source_identifier: 's1',
    ignored_at: '2026-04-01T10:00:00Z',
    ignored_by: 'u1',
    ignored_by_username: 'ada',
    reason: 'deleted by user',
    created_at: '2026-04-01T10:00:00Z',
    ...overrides,
  };
}

function serveFiles(files: unknown[], total = files.length) {
  ignoredFilesService.list.mockImplementation(() => ok({ ignored_files: files, total }));
}

const grid = () => screen.findByRole('grid', { name: 'Ignored files' });
const lastQuery = () => ignoredFilesService.list.mock.calls.at(-1)?.[0];

beforeEach(() => {
  resetIntakeState();
  serveDefaults();
  sourcesService.list.mockImplementation(() => ok([source('s1', { name: 'Office cloud' })]));
  serveFiles([file('a'), file('b', { reason: 'too large', source_type: 's3', source_identifier: 'zz' })]);
  ignoredFilesService.stats.mockImplementation(() =>
    ok({ total_ignored_files: 2, by_source_type: [], total_size_bytes: 2097152, most_recent_ignored_at: '2026-04-01T10:00:00Z' }),
  );
});

describe('Ignored section (ported from IgnoredFilesPage)', () => {
  it('renders the board with name, source, size, date and reason', async () => {
    renderIntake(<IgnoredSection />);
    const table = await grid();
    const row = within(table).getByRole('row', { name: /a\.pdf/ });
    expect(row).toHaveTextContent('WebDAV · Office cloud');
    expect(row).toHaveTextContent('1 MB');
    expect(row).toHaveTextContent('deleted by user');
  });

  it('keeps source, size, date and reason reachable on a phone', async () => {
    const original = window.matchMedia;
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      configurable: true,
      value: createResponsiveMatchMediaMock({ 'max-width: 719px': true }),
    });
    try {
      renderIntake(<IgnoredSection />);
      const table = await grid();
      expect(within(table).queryByRole('columnheader', { name: 'Reason' })).not.toBeInTheDocument();
      const row = within(table).getByRole('row', { name: /a\.pdf/ });
      expect(within(row).getByText('deleted by user')).toBeVisible();
      expect(row).toHaveAccessibleDescription(/Source: WebDAV · Office cloud; Size: 1 MB; Ignored: .+; Reason: deleted by user;/);
    } finally {
      Object.defineProperty(window, 'matchMedia', {
        writable: true,
        configurable: true,
        value: original,
      });
    }
  });

  it('reads the source filter from the URL and sends it to the server', async () => {
    renderIntake(<IgnoredSection />, { path: '/intake?section=ignored&sourceType=webdav&sourceName=My%20WebDAV%20Server&sourceId=source-123' });
    await grid();
    expect(lastQuery()).toMatchObject({ source_type: 'webdav', source_identifier: 'source-123', limit: 25, offset: 0 });
  });

  it('shows the source name, including special characters, in the breadcrumbs', async () => {
    renderIntake(<IgnoredSection />, { path: '/intake?section=ignored&sourceName=My%20Server%20%26%20More!&sourceId=s1' });
    const crumbs = screen.getByRole('navigation', { name: 'Breadcrumbs' });
    expect(within(crumbs).getByRole('link', { name: 'Connections' })).toHaveAttribute('href', '/sources?section=connections');
    expect(within(crumbs).getByText('My Server & More!')).toBeInTheDocument();
    expect(within(crumbs).getByText('Ignored files')).toHaveAttribute('aria-current', 'page');
  });

  it('formats sizes in the summary', async () => {
    renderIntake(<IgnoredSection />);
    const summary = await screen.findByRole('group', { name: 'Ignored files summary' });
    expect(summary).toHaveTextContent('2 MB');
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(1024, 2)).toBe('1 KB');
    expect(formatBytes(1073741824, 2)).toBe('1 GB');
  });

  it('labels source types for display', async () => {
    renderIntake(<IgnoredSection />);
    const table = await grid();
    expect(within(table).getByRole('row', { name: /b\.pdf/ })).toHaveTextContent('S3 · zz');
  });

  it('searches file names on the server and returns to page 1', async () => {
    const user = userEvent.setup();
    renderIntake(<IgnoredSection />);
    await grid();
    await user.type(screen.getByRole('searchbox', { name: 'Search file names' }), 'document');
    await waitFor(() => expect(lastQuery()).toMatchObject({ filename: 'document', offset: 0 }));
  });

  it('filters by reason on the current page', async () => {
    const user = userEvent.setup();
    renderIntake(<IgnoredSection />);
    const table = await grid();
    await user.click(screen.getByRole('button', { name: /reason/i }));
    await user.click(screen.getByRole('option', { name: 'too large' }));
    expect(within(table).queryByRole('row', { name: /a\.pdf/ })).not.toBeInTheDocument();
    expect(within(table).getByRole('row', { name: /b\.pdf/ })).toBeInTheDocument();
  });

  it('clears the source filter', async () => {
    const user = userEvent.setup();
    renderIntake(<IgnoredSection />, { path: '/intake?section=ignored&sourceType=s3&sourceId=s1&sourceName=Bucket' });
    await grid();
    await user.click(screen.getByRole('button', { name: 'Show all sources' }));
    expect(screen.getByRole('status', { name: 'location', hidden: true })).toHaveTextContent('/intake?section=ignored');
    await waitFor(() => expect(lastQuery()?.source_identifier).toBeUndefined());
    expect(lastQuery()?.source_type).toBeUndefined();
  });

  it('un-ignores the selected files after confirmation', async () => {
    const user = userEvent.setup();
    renderIntake(<IgnoredSection />);
    const table = await grid();
    await user.click(within(table).getByRole('checkbox', { name: 'Select All' }));
    await user.click(within(screen.getByRole('toolbar', { name: 'Bulk actions' })).getByRole('button', { name: 'Un-ignore' }));
    const dialog = screen.getByRole('alertdialog', { name: 'Un-ignore 2 files?' });
    expect(ignoredFilesService.bulkRemove).not.toHaveBeenCalled();
    await user.click(within(dialog).getByRole('button', { name: 'Un-ignore' }));
    await waitFor(() => expect(ignoredFilesService.bulkRemove).toHaveBeenCalledWith(['a', 'b']));
  });

  it('un-ignores one file after confirmation', async () => {
    const user = userEvent.setup();
    renderIntake(<IgnoredSection />);
    const table = await grid();
    await user.click(within(table).getByRole('button', { name: 'Un-ignore a.pdf' }));
    await user.click(within(screen.getByRole('alertdialog', { name: 'Un-ignore “a.pdf”?' })).getByRole('button', { name: 'Un-ignore' }));
    await waitFor(() => expect(ignoredFilesService.remove).toHaveBeenCalledWith('a'));
  });

  it('pages through results with the page size', async () => {
    serveFiles([file('a')], 100);
    const user = userEvent.setup();
    renderIntake(<IgnoredSection />);
    await grid();
    await user.click(screen.getByRole('button', { name: /next/i }));
    await waitFor(() => expect(lastQuery()).toMatchObject({ limit: 25, offset: 25 }));
  });

  it('shows the empty state', async () => {
    serveFiles([]);
    renderIntake(<IgnoredSection />);
    expect(await screen.findByRole('heading', { name: 'Nothing is ignored' })).toBeInTheDocument();
  });

  it('shows a retryable error', async () => {
    ignoredFilesService.list.mockImplementation(() => Promise.reject(new Error('down')));
    renderIntake(<IgnoredSection />);
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load ignored files.');
    expect(screen.getByRole('button', { name: 'Retry' })).toBeInTheDocument();
  });
});
