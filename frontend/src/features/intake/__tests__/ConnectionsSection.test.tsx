import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../services/api', async () => (await import('./intakeMocks')).apiModule);

import { ConnectionsSection } from '../connections/ConnectionsSection';
import { ignoredFilesHref, sourceTypeLabel } from '../shared/sourceTypes';
import { acknowledge, isLit } from '../../board/litStore';
import i18n from 'i18next';
import { apiError, ok, queueService, serveDefaults, sourceErrorService, sourcesService } from './intakeMocks';
import { renderIntake, resetIntakeState, source } from './intakeTestUtils';

const SOURCES = [
  source('s1', { name: 'Office cloud' }),
  source('s2', {
    name: 'Archive bucket',
    source_type: 's3',
    config: { bucket_name: 'archive', access_key_id: 'AKIAABCDEFGHWXYZ', secret_access_key: 'x', watch_folders: ['docs/'], auto_sync: false },
    status: 'error',
    last_error: 'Access denied',
    last_error_at: '2026-05-01T10:00:00Z',
  }),
  source('s3', { name: 'Scanner share', source_type: 'local_folder', config: { watch_folders: ['/scans'] }, enabled: false }),
];

function serveSources(list: unknown = SOURCES) {
  sourcesService.list.mockImplementation(() => (list instanceof Error ? Promise.reject(list) : ok(list)));
}

async function board() {
  return screen.findByRole('grid', { name: 'Connections' });
}

async function openRow(user: ReturnType<typeof userEvent.setup>, name: string) {
  const grid = await board();
  await user.click(within(grid).getByRole('rowheader', { name: new RegExp(name) }));
  return screen.findByRole('dialog', { name });
}

beforeEach(() => {
  resetIntakeState();
  serveDefaults();
  serveSources();
});

describe('Connections board', () => {
  it('lists every connection with type, status, last sync, files and next sync', async () => {
    renderIntake(<ConnectionsSection />);
    const grid = await board();
    const heads = within(grid).getAllByRole('columnheader').map((h) => h.textContent?.replace(/[▲▼]/g, ''));
    expect(heads).toEqual(['Name', 'Type', 'Status', 'Last sync', 'Files', 'Next']);
    const office = within(grid).getByRole('row', { name: /Office cloud/ });
    expect(within(office).getByText('WebDAV')).toBeInTheDocument();
    expect(within(office).getByText(/^healthy$/i)).toBeInTheDocument();
    expect(within(office).getByText('12')).toBeInTheDocument();
    expect(within(grid).getByRole('row', { name: /Archive bucket/ })).toHaveTextContent(/error/i);
    expect(within(grid).getByRole('row', { name: /Scanner share/ })).toHaveTextContent(/off/i);
  });

  it('marks a connection in error and shows a Changed tag until it is opened', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const grid = await board();
    const row = within(grid).getByRole('row', { name: /Archive bucket/ });
    expect(row).toHaveAttribute('data-changed', 'true');
    expect(within(row).getByText('Changed')).toBeInTheDocument();
    expect(isLit('source', 's2')).toBe(true);
    expect(within(grid).getByRole('row', { name: /Office cloud/ })).not.toHaveAttribute('data-changed');
    await openRow(user, 'Archive bucket');
    expect(isLit('source', 's2')).toBe(false);
  });

  it('does not mark the same failure again after it was acknowledged', async () => {
    const first = renderIntake(<ConnectionsSection />);
    await board();
    acknowledge('source', 's2');
    first.unmount();
    renderIntake(<ConnectionsSection />);
    await board();
    expect(isLit('source', 's2')).toBe(false);
  });

  it('sorts by name when the column head is pressed', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const grid = await board();
    const names = () => within(grid).getAllByRole('rowheader').map((c) => c.textContent);
    expect(names()).toEqual(['Archive bucketChanged', 'Office cloud', 'Scanner share']);
    await user.click(within(grid).getByRole('columnheader', { name: /Name/ }));
    expect(names()).toEqual(['Scanner share', 'Office cloud', 'Archive bucketChanged']);
  });

  it('shows the empty state with an add button', async () => {
    serveSources([]);
    renderIntake(<ConnectionsSection />);
    expect(await screen.findByRole('heading', { name: 'No connections yet' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Add your first connection' })).toBeInTheDocument();
  });

  it('shows a retryable error when the list cannot be loaded', async () => {
    serveSources(new Error('down'));
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load connections.');
    serveSources();
    await user.click(screen.getByRole('button', { name: 'Retry' }));
    expect(await board()).toBeInTheDocument();
  });

  it('opens the connection form from "Add connection"', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    await board();
    await user.click(screen.getByRole('button', { name: 'Add connection' }));
    expect(screen.getByRole('dialog', { name: 'Add connection' })).toBeInTheDocument();
  });

  it('opens the connection form from ?new=1 and drops the param', async () => {
    renderIntake(<ConnectionsSection />, { path: '/sources?section=connections&new=1' });
    expect(await screen.findByRole('dialog', { name: 'Add connection' })).toBeInTheDocument();
    await waitFor(() =>
      expect(screen.getByRole('status', { name: 'location', hidden: true }).textContent).toBe('/sources?section=connections'),
    );
  });

  it('offers Pause OCR to admins only, and pauses', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    await user.click(await screen.findByRole('button', { name: 'Pause OCR' }));
    await waitFor(() => expect(queueService.pauseOcr).toHaveBeenCalled());
  });

  it('hides the OCR switch from regular users', async () => {
    renderIntake(<ConnectionsSection />, { role: 'user' });
    await board();
    expect(screen.queryByRole('button', { name: /pause ocr|resume ocr/i })).not.toBeInTheDocument();
  });
});

describe('Connection details panel', () => {
  it('shows location, masked sign-in, schedule and counts', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    expect(within(panel).getByRole('group', { name: 'Connection' })).toHaveTextContent('https://cloud.example.com');
    expect(within(panel).getByRole('group', { name: 'Connection' })).toHaveTextContent('ada · password set');
    expect(panel).not.toHaveTextContent('secret');
    expect(within(panel).getByRole('group', { name: 'Schedule' })).toHaveTextContent('every 60 min');
    expect(within(panel).getByRole('group', { name: 'Counts' })).toHaveTextContent('2 KB');
  });

  it('lists recent errors with PascalCase types mapped to words and marks', async () => {
    sourceErrorService.listFailures.mockImplementation(() =>
      ok([
        {
          id: 'e1',
          source_type: 'WebDAV',
          source_name: 'Office cloud',
          resource_path: '/Documents/big',
          error_type: 'PermissionDenied',
          error_severity: 'High',
          failure_count: 3,
          consecutive_failures: 3,
          first_failure_at: '2026-05-01T09:00:00Z',
          last_failure_at: '2026-05-01T10:00:00Z',
          next_retry_at: null,
          error_message: '403 Forbidden',
          http_status_code: 403,
          user_excluded: false,
          user_notes: null,
          resolved: false,
          diagnostic_summary: {},
        },
      ]),
    );
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    const list = await within(panel).findByRole('list', { name: 'Recent errors' });
    expect(within(list).getByText('Permission denied')).toBeInTheDocument();
    expect(within(list).getByText('Severity: High')).toBeInTheDocument();
    expect(within(list).getByText(/^error$/i)).toBeInTheDocument();
    expect(within(list).getByText('/Documents/big')).toBeInTheDocument();
    expect(sourceErrorService.listFailures).toHaveBeenCalledWith({ source_id: 's1', limit: 5 });
  });

  it('says so when there are no recent errors', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    expect(await within(panel).findByText('No recent errors.')).toBeInTheDocument();
  });

  it.each([
    ['Sync now', 'triggerSync'],
    ['Deep scan', 'triggerDeepScan'],
    ['Test connection', 'testConnection'],
    ['Check health', 'validate'],
  ] as const)('"%s" calls sourcesService.%s', async (label, method) => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    await user.click(within(panel).getByRole('button', { name: label }));
    await waitFor(() => expect(sourcesService[method]).toHaveBeenCalled());
    if (method === 'testConnection') {
      expect(sourcesService.testConnection).toHaveBeenCalledWith({ source_type: 'webdav', config: SOURCES[0].config });
    } else {
      expect(sourcesService[method]).toHaveBeenCalledWith('s1');
    }
  });

  it('tests a saved connection whose secret the server keeps, by its id', async () => {
    const redacted = source('s1', {
      name: 'Office cloud',
      config: { server_url: 'https://cloud.example.com', username: 'ada', has_password: true, watch_folders: ['/Documents'] },
    });
    serveSources([redacted]);
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    expect(within(panel).getByText(/password set/)).toBeInTheDocument();
    await user.click(within(panel).getByRole('button', { name: 'Test connection' }));
    await waitFor(() => expect(sourcesService.testConnection).toHaveBeenCalled());
    expect(sourcesService.testConnection).toHaveBeenCalledWith({
      source_type: 'webdav',
      config: redacted.config,
      source_id: 's1',
    });
  });

  it('explains an invalid configuration when testing the saved connection', async () => {
    sourcesService.testConnection.mockImplementation(() => Promise.reject(apiError(400, 'SOURCE_CONFIG_INVALID', 'bad')));
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    await user.click(within(panel).getByRole('button', { name: 'Test connection' }));
    expect(await screen.findByText('Some settings are invalid. Check the values and try again.')).toBeInTheDocument();
  });

  it('offers Stop sync instead of Sync now while syncing', async () => {
    serveSources([source('s1', { name: 'Office cloud', status: 'syncing' })]);
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    expect(within(panel).queryByRole('button', { name: 'Sync now' })).not.toBeInTheDocument();
    await user.click(within(panel).getByRole('button', { name: 'Stop sync' }));
    await waitFor(() => expect(sourcesService.stopSync).toHaveBeenCalledWith('s1'));
  });

  it('turns a connection off and on', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    await user.click(within(panel).getByRole('button', { name: 'Disable' }));
    await waitFor(() => expect(sourcesService.update).toHaveBeenCalledWith('s1', { enabled: false }));
  });

  it('opens the form with the connection loaded from Edit', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    await user.click(within(panel).getByRole('button', { name: 'Edit' }));
    const form = screen.getByRole('dialog', { name: 'Edit connection' });
    expect(within(form).getByRole('textbox', { name: /^name/i })).toHaveValue('Office cloud');
  });

  it('deletes only after confirmation', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    await user.click(within(panel).getByRole('button', { name: 'Delete' }));
    const confirm = screen.getByRole('alertdialog', { name: 'Delete “Office cloud”?' });
    expect(sourcesService.remove).not.toHaveBeenCalled();
    await user.click(within(confirm).getByRole('button', { name: 'Delete connection' }));
    await waitFor(() => expect(sourcesService.remove).toHaveBeenCalledWith('s1'));
    await waitFor(() => expect(screen.queryByRole('alertdialog', { name: 'Delete “Office cloud”?' })).not.toBeInTheDocument());
    expect(screen.queryByRole('dialog', { name: 'Office cloud' })).not.toBeInTheDocument();
  });

  it('cancels a delete without calling the server', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    await user.click(within(panel).getByRole('button', { name: 'Delete' }));
    await user.click(within(screen.getByRole('alertdialog', { name: /Delete/ })).getByRole('button', { name: 'Cancel' }));
    expect(sourcesService.remove).not.toHaveBeenCalled();
  });

  it('explains why a delete failed while syncing', async () => {
    sourcesService.remove.mockImplementation(() => Promise.reject(apiError(409, 'SOURCE_SYNC_IN_PROGRESS', 'busy')));
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    await user.click(within(panel).getByRole('button', { name: 'Delete' }));
    await user.click(within(screen.getByRole('alertdialog', { name: /Delete/ })).getByRole('button', { name: 'Delete connection' }));
    expect(await screen.findByText('Stop the sync before deleting this connection.')).toBeInTheDocument();
  });
});

describe('Sync options (ported from SourcesPage.sync-functionality)', () => {
  it('offers both Sync now and Deep scan', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    expect(within(panel).getByRole('button', { name: 'Sync now' })).toBeEnabled();
    expect(within(panel).getByRole('button', { name: 'Deep scan' })).toBeEnabled();
  });

  it('allows deep scan for WebDAV only and explains why elsewhere', async () => {
    serveSources([source('s9', { name: 'Local scans', source_type: 'local_folder', config: { watch_folders: ['/scans'] } })]);
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Local scans');
    const deep = within(panel).getByRole('button', { name: 'Deep scan' });
    expect(deep).toBeDisabled();
    expect(deep).toHaveAccessibleDescription(/only available for WebDAV/);
    expect(within(panel).getByRole('button', { name: 'Sync now' })).toBeEnabled();
  });

  it('uses the sync, deep scan and stop services with the connection id', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    await user.click(within(panel).getByRole('button', { name: 'Sync now' }));
    await waitFor(() => expect(sourcesService.triggerSync).toHaveBeenCalledWith('s1'));
    await user.click(within(panel).getByRole('button', { name: 'Deep scan' }));
    await waitFor(() => expect(sourcesService.triggerDeepScan).toHaveBeenCalledWith('s1'));
  });

  it('never calls deep scan for a non-WebDAV connection', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Archive bucket');
    await user.click(within(panel).getByRole('button', { name: 'Deep scan' }));
    expect(sourcesService.triggerDeepScan).not.toHaveBeenCalled();
  });

  it('gives clear feedback: started, and already syncing', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    await user.click(within(panel).getByRole('button', { name: 'Sync now' }));
    expect(await screen.findByText('Sync started')).toBeInTheDocument();
    sourcesService.triggerDeepScan.mockImplementation(() => Promise.reject(apiError(409)));
    await user.click(within(panel).getByRole('button', { name: 'Deep scan' }));
    expect(await screen.findByText('This connection is already syncing.')).toBeInTheDocument();
  });
});

describe('Ignored files link (ported from SourcesPage.ignored-files)', () => {
  const t = i18n.t.bind(i18n);
  const params = (href: string) => new URL(href, 'http://x').searchParams;

  it('builds the Ignored section link for each source type', () => {
    for (const [type, name, id] of [['webdav', 'WebDAV Server', 'source-1'], ['s3', 'S3 Bucket', 'source-2'], ['local_folder', 'Local Documents', 'source-3']]) {
      const p = params(ignoredFilesHref({ id, name, source_type: type }));
      expect(p.get('section')).toBe('ignored');
      expect(p.get('sourceType')).toBe(type);
      expect(p.get('sourceName')).toBe(name);
      expect(p.get('sourceId')).toBe(id);
    }
  });

  it('encodes special characters and unicode in the source name', () => {
    const href = ignoredFilesHref({ id: 's', name: 'My WebDAV & More!', source_type: 'webdav' });
    expect(href).not.toContain('& More');
    expect(params(href).get('sourceName')).toBe('My WebDAV & More!');
    expect(params(ignoredFilesHref({ id: 's', name: 'Документы', source_type: 'local_folder' })).get('sourceName')).toBe('Документы');
  });

  it('includes all three source parameters', () => {
    const href = ignoredFilesHref({ id: 'source-123', name: 'Test Source', source_type: 'webdav' });
    for (const key of ['sourceType', 'sourceName', 'sourceId']) expect(params(href).has(key)).toBe(true);
  });

  it('labels the known source types and falls back for unknown or missing ones', () => {
    expect(sourceTypeLabel(t, 'webdav')).toBe('WebDAV');
    expect(sourceTypeLabel(t, 's3')).toBe('S3');
    expect(sourceTypeLabel(t, 'local_folder')).toBe('Local folder');
    expect(sourceTypeLabel(t, 'ftp')).toBe('ftp');
    expect(sourceTypeLabel(t, null)).toBe('Unknown');
  });

  it('has an accessible "Ignored files" button in the panel', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    expect(within(panel).getByRole('button', { name: 'Ignored files' })).toBeInTheDocument();
  });

  it('places it between Edit and Delete', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    const names = within(panel).getAllByRole('button').map((b) => b.textContent);
    const edit = names.indexOf('Edit');
    const ignored = names.indexOf('Ignored files');
    const del = names.indexOf('Delete');
    expect(edit).toBeLessThan(ignored);
    expect(ignored).toBeLessThan(del);
  });

  it.each([
    ['idle', 'Office cloud'],
    ['error', 'Archive bucket'],
  ])('is available for a connection that is %s', async (_state, name) => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, name);
    expect(within(panel).getByRole('button', { name: 'Ignored files' })).toBeEnabled();
  });

  it('is available for a syncing connection', async () => {
    serveSources([source('s1', { name: 'Office cloud', status: 'syncing' })]);
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    expect(within(panel).getByRole('button', { name: 'Ignored files' })).toBeEnabled();
  });

  it('is available for a disabled connection', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Scanner share');
    expect(within(panel).getByRole('button', { name: 'Ignored files' })).toBeEnabled();
  });

  it('navigates to the filtered Ignored section', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    await user.click(within(panel).getByRole('button', { name: 'Ignored files' }));
    const location = screen.getByRole('status', { name: 'location', hidden: true });
    expect(location.textContent).toContain('/intake?section=ignored');
    expect(location.textContent).toContain('sourceId=s1');
    expect(location.textContent).toContain('sourceName=Office+cloud');
  });

  it('works from the keyboard', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const panel = await openRow(user, 'Office cloud');
    within(panel).getByRole('button', { name: 'Ignored files' }).focus();
    await user.keyboard('{Enter}');
    expect(screen.getByRole('status', { name: 'location', hidden: true }).textContent).toContain('section=ignored');
  });
});

describe('Connections health and deep links', () => {
  it("keeps a failing connection's row on one line and gives its last error in plain words on its status", async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />);
    const grid = await board();
    const row = within(grid).getByRole('row', { name: /Archive bucket/ });
    expect(row).not.toHaveTextContent(/refused the sign-in/i);
    await user.click(document.body); // a pointer interaction first, as React Aria tooltips expect
    await user.hover(row.querySelector('[data-reason]') as HTMLElement);
    expect(await screen.findByRole('tooltip')).toHaveTextContent(/server refused the sign-in/i);
    expect(within(grid).getByRole('row', { name: /Office cloud/ }).querySelector('[data-reason]')).toBeNull();
  });

  it('lists a health warning under the name and its recommendation in the panel', async () => {
    const user = userEvent.setup();
    serveSources([
      source('w1', {
        name: 'Office cloud',
        validation_status: 'warning',
        validation_issues: JSON.stringify([
          { message: 'The server host could not be resolved', recommendation: 'Check server URL and network', severity: 'warning' },
        ]),
      }),
    ]);
    renderIntake(<ConnectionsSection />);
    const grid = await board();
    const row = within(grid).getByRole('row', { name: /Office cloud/ });
    await user.click(document.body); // a pointer interaction first, as React Aria tooltips expect
    await user.hover(row.querySelector('[data-reason]') as HTMLElement);
    expect(await screen.findByRole('tooltip')).toHaveTextContent(/Can't reach the server/);
    await user.unhover(row.querySelector('[data-reason]') as HTMLElement);
    const panel = await openRow(user, 'Office cloud');
    expect(within(panel).getByRole('heading', { name: 'Health check' })).toBeInTheDocument();
    expect(within(panel).getByText('Check server URL and network')).toBeInTheDocument();
  });

  it('opens the connection named by ?source= and drops the param on close', async () => {
    const user = userEvent.setup();
    renderIntake(<ConnectionsSection />, { path: '/intake?section=connections&source=s2' });
    const dialog = await screen.findByRole('dialog', { name: 'Archive bucket' });
    expect(screen.getByRole('status', { name: 'location', hidden: true })).toHaveTextContent('source=s2');
    await user.click(within(dialog).getByRole('button', { name: /close/i }));
    await waitFor(() =>
      expect(screen.getByRole('status', { name: 'location', hidden: true }).textContent).not.toContain('source='),
    );
  });

  it('ignores a ?source= that matches no connection', async () => {
    renderIntake(<ConnectionsSection />, { path: '/intake?section=connections&source=nope' });
    await board();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});

describe('closing the details panel', () => {
  it('keeps the connection on screen while the panel slides out', async () => {
    // Pretend the exit animation is still running, as it does in a browser.
    const running = [{ finished: new Promise(() => {}) }] as unknown as Animation[];
    const original = Element.prototype.getAnimations;
    Element.prototype.getAnimations = function getAnimations(this: Element) {
      return this.hasAttribute('data-exiting') ? running : [];
    };
    try {
      const user = userEvent.setup();
      renderIntake(<ConnectionsSection />);
      const panel = await openRow(user, 'Archive bucket');
      await user.click(within(panel).getByRole('button', { name: /close/i }));
      await waitFor(() => expect(document.querySelector('[data-exiting]')).not.toBeNull());
      expect(within(document.querySelector('[data-exiting]') as HTMLElement).getByText('Archive bucket')).toBeInTheDocument();
    } finally {
      Element.prototype.getAnimations = original;
    }
  });
});
