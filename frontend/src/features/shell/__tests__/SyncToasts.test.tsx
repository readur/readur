import { act, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { SourceResponse } from '../../../services/api';
import { MockSyncProgressManager } from '../../../services/syncProgress/MockSyncProgressManager';
import { ToastProvider } from '../../../ui';
import { SyncToasts } from '../SyncToasts';

const source = (status: string) => ({ id: 's1', name: 'Nextcloud', source_type: 'webdav', enabled: true, status }) as unknown as SourceResponse;

const progress = {
  source_id: 's1',
  phase: 'processing_files',
  phase_description: 'Processing files',
  elapsed_time_secs: 30,
  directories_found: 4,
  directories_processed: 4,
  files_found: 412,
  files_processed: 240,
  bytes_processed: 0,
  processing_rate_files_per_sec: 8,
  files_progress_percent: 58.3,
  estimated_time_remaining_secs: 20,
  current_directory: '/',
  current_file: null,
  errors: 0,
  warnings: 0,
  is_active: true,
};

describe('SyncToasts', () => {
  it('shows a progress toast while a source syncs and removes it when the sync ends', async () => {
    const manager = new MockSyncProgressManager();
    const view = render(
      <ToastProvider>
        <SyncToasts sources={[source('syncing')]} manager={manager} />
      </ToastProvider>,
    );
    expect(await screen.findByText('Syncing Nextcloud')).toBeInTheDocument();
    await waitFor(() => expect(manager.isConnected()).toBe(true));
    act(() => manager.simulateProgress(progress));
    expect(await screen.findByText('240 of 412 files')).toBeInTheDocument();
    expect(screen.getByRole('progressbar', { name: 'Syncing Nextcloud' })).toHaveAttribute('aria-valuenow', '58');

    view.rerender(
      <ToastProvider>
        <SyncToasts sources={[source('idle')]} manager={manager} />
      </ToastProvider>,
    );
    await waitFor(() => expect(screen.queryByText('Syncing Nextcloud')).not.toBeInTheDocument());
  });

  it('shows nothing when no source is syncing', () => {
    render(
      <ToastProvider>
        <SyncToasts sources={[source('idle')]} manager={new MockSyncProgressManager()} />
      </ToastProvider>,
    );
    expect(screen.queryByText(/Syncing/)).not.toBeInTheDocument();
  });
});
