import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../services/api', async () => (await import('./intakeMocks')).apiModule);

import { WatchSection } from '../watch/WatchSection';
import { SYSTEM_WATCH } from '../watch/watchConfig';
import { ok, queueService, serveDefaults, userWatchService } from './intakeMocks';
import { renderIntake, resetIntakeState } from './intakeTestUtils';

beforeEach(() => {
  resetIntakeState();
  serveDefaults();
  queueService.getStats.mockImplementation(() =>
    ok({ pending_count: 7, processing_count: 2, failed_count: 3, completed_today: 41, avg_wait_time_minutes: 12, oldest_pending_minutes: 130 }),
  );
});

describe('Watch folder section', () => {
  it('summarizes the processing queue in labelled mono cells', async () => {
    renderIntake(<WatchSection />);
    const queue = await screen.findByRole('group', { name: 'Processing queue' });
    for (const [label, value] of [['Waiting', '7'], ['Processing', '2'], ['Failed', '3'], ['Done today', '41'], ['Average wait', '12m'], ['Oldest waiting', '2h 10m']]) {
      expect(within(queue).getByText(label).nextElementSibling).toHaveTextContent(value);
    }
  });

  it('lists the server watch folder with its status and settings', async () => {
    renderIntake(<WatchSection />);
    const grid = await screen.findByRole('grid', { name: 'Watched folders' });
    const row = within(grid).getByRole('row', { name: new RegExp(SYSTEM_WATCH.folder.replace('.', '\\.')) });
    expect(row).toHaveTextContent(/healthy/i);
    expect(row).toHaveTextContent('every 30 s · files up to 24 h old');
    expect(screen.getByText(SYSTEM_WATCH.allowedTypes.map((x) => `.${x}`).join(' '))).toBeInTheDocument();
  });

  it('does not ask for a personal folder when per-user watch is off', async () => {
    renderIntake(<WatchSection />);
    await screen.findByRole('grid', { name: 'Watched folders' });
    expect(userWatchService.getUserWatchDirectory).not.toHaveBeenCalled();
    expect(screen.getAllByRole('row')).toHaveLength(2);
  });

  it('adds the personal watch folder when per-user watch is on', async () => {
    renderIntake(<WatchSection />, { perUserWatch: true });
    const grid = await screen.findByRole('grid', { name: 'Watched folders' });
    await waitFor(() => expect(within(grid).getByRole('row', { name: /\/watch\/ada/ })).toBeInTheDocument());
    expect(userWatchService.getUserWatchDirectory).toHaveBeenCalledWith('1');
  });

  it('offers to create a missing personal folder', async () => {
    userWatchService.getUserWatchDirectory.mockImplementation(() =>
      ok({ user_id: '1', username: 'ada', watch_directory_path: '/watch/ada', exists: false, enabled: true }),
    );
    const user = userEvent.setup();
    renderIntake(<WatchSection />, { perUserWatch: true });
    await user.click(await screen.findByRole('button', { name: 'Create my watch folder' }));
    await waitFor(() => expect(userWatchService.createUserWatchDirectory).toHaveBeenCalledWith('1'));
    expect(await screen.findByText('Watch folder created')).toBeInTheDocument();
  });

  it('retries failed jobs only after confirmation', async () => {
    const user = userEvent.setup();
    renderIntake(<WatchSection />);
    await user.click(await screen.findByRole('button', { name: 'Retry 3 failed jobs' }));
    const dialog = screen.getByRole('alertdialog', { name: 'Retry failed jobs?' });
    expect(queueService.requeueFailed).not.toHaveBeenCalled();
    await user.click(within(dialog).getByRole('button', { name: 'Retry jobs' }));
    await waitFor(() => expect(queueService.requeueFailed).toHaveBeenCalled());
    expect(await screen.findByText('2 jobs queued again')).toBeInTheDocument();
  });

  it('shows an error when the queue cannot be loaded', async () => {
    queueService.getStats.mockImplementation(() => Promise.reject(new Error('down')));
    renderIntake(<WatchSection />);
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not load the processing queue.');
  });

  it('explains the server folder to regular users', async () => {
    renderIntake(<WatchSection />, { role: 'User' });
    expect(await screen.findByText('The server folder is set by your administrator and applies to everyone.')).toBeInTheDocument();
  });
});
