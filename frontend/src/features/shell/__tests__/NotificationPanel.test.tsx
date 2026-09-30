import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import api from '../../../services/api';
import { installStorage, renderShell, setMedia, type SeedNotification } from './shellTestUtils';

vi.mock('../../../services/api', () => ({
  default: { get: vi.fn() },
  documentService: { enhancedSearch: vi.fn() },
}));

vi.mock('date-fns', () => ({
  formatDistanceToNow: vi.fn(() => '2 minutes ago'),
}));

const seed: SeedNotification[] = [
  { type: 'success', title: 'Upload complete', message: '3 files added' },
  { type: 'error', title: 'OCR failed', message: 'scan.pdf could not be read' },
];

const openPanel = async (user: ReturnType<typeof userEvent.setup>, name: RegExp | string) => {
  await user.click(screen.getByRole('button', { name }));
  return screen.findByRole('dialog', { name: 'Notifications' });
};

beforeEach(() => {
  setMedia();
  installStorage();
  vi.mocked(api.get).mockReturnValue(new Promise(() => {}) as never);
});

describe('alerts button', () => {
  it('is named "Notifications" and shows no badge when nothing is unread', () => {
    renderShell();
    const button = screen.getByRole('button', { name: 'Notifications' });
    expect(button).toBeInTheDocument();
    expect(within(button).queryByText(/\d/)).not.toBeInTheDocument();
  });

  it('shows the unread count in its name and badge', async () => {
    renderShell({ notifications: seed });
    const button = await screen.findByRole('button', { name: 'Notifications, 2 unread' });
    expect(within(button).getByText('2')).toBeInTheDocument();
  });

  it('keeps the panel closed until pressed', () => {
    renderShell({ notifications: seed });
    expect(screen.queryByRole('dialog', { name: 'Notifications' })).not.toBeInTheDocument();
  });
});

describe('notification panel', () => {
  it('opens with a heading and the unread count', async () => {
    const user = userEvent.setup();
    renderShell({ notifications: seed });
    const panel = await openPanel(user, /Notifications, 2 unread/);
    expect(within(panel).getByRole('heading', { name: 'Notifications' })).toBeInTheDocument();
    expect(within(panel).getByText('2 unread')).toBeInTheDocument();
  });

  it('shows the empty state when there are no notifications', async () => {
    const user = userEvent.setup();
    renderShell();
    const panel = await openPanel(user, 'Notifications');
    expect(within(panel).getByText('No notifications')).toBeInTheDocument();
    expect(within(panel).queryByRole('button', { name: 'Clear all' })).not.toBeInTheDocument();
  });

  it('lists each notification with its type as text, message and time', async () => {
    const user = userEvent.setup();
    renderShell({ notifications: seed });
    const panel = await openPanel(user, /2 unread/);
    const list = within(panel).getByRole('list', { name: 'Notifications' });
    expect(within(list).getAllByRole('listitem')).toHaveLength(2);
    const failed = within(list).getByRole('button', { name: 'New: Error: OCR failed' });
    expect(failed).toHaveAccessibleDescription('scan.pdf could not be read');
    expect(within(list).getByRole('button', { name: 'New: Success: Upload complete' })).toBeInTheDocument();
    expect(within(list).getAllByText('2 minutes ago')).toHaveLength(2);
  });

  it('marks one notification as read when pressed', async () => {
    const user = userEvent.setup();
    renderShell({ notifications: seed });
    const panel = await openPanel(user, /2 unread/);
    await user.click(within(panel).getByRole('button', { name: 'New: Error: OCR failed' }));
    expect(within(panel).getByRole('button', { name: 'Error: OCR failed' })).toBeInTheDocument();
    expect(within(panel).getByText('1 unread')).toBeInTheDocument();
    // The page behind the open popover is hidden from assistive tech.
    expect(screen.getByRole('button', { name: 'Notifications, 1 unread', hidden: true })).toBeInTheDocument();
  });

  it('marks all as read', async () => {
    const user = userEvent.setup();
    renderShell({ notifications: seed });
    const panel = await openPanel(user, /2 unread/);
    await user.click(within(panel).getByRole('button', { name: 'Mark all as read' }));
    expect(within(panel).queryByText(/unread/)).not.toBeInTheDocument();
    expect(within(panel).getByRole('button', { name: 'Mark all as read' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Notifications', hidden: true })).toBeInTheDocument();
  });

  it('dismisses one notification', async () => {
    const user = userEvent.setup();
    renderShell({ notifications: seed });
    const panel = await openPanel(user, /2 unread/);
    await user.click(within(panel).getByRole('button', { name: 'Dismiss OCR failed' }));
    expect(within(panel).queryByText('OCR failed')).not.toBeInTheDocument();
    expect(within(panel).getAllByRole('listitem')).toHaveLength(1);
  });

  it('clears all notifications', async () => {
    const user = userEvent.setup();
    renderShell({ notifications: seed });
    const panel = await openPanel(user, /2 unread/);
    await user.click(within(panel).getByRole('button', { name: 'Clear all' }));
    expect(within(panel).getByText('No notifications')).toBeInTheDocument();
  });

  it('closes from its close button and from Escape, returning focus to the bell', async () => {
    const user = userEvent.setup();
    renderShell({ notifications: seed });
    let panel = await openPanel(user, /2 unread/);
    await user.click(within(panel).getByRole('button', { name: 'Close notifications' }));
    await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Notifications' })).not.toBeInTheDocument());

    panel = await openPanel(user, /2 unread/);
    expect(panel).toBeInTheDocument();
    await user.keyboard('{Escape}');
    await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Notifications' })).not.toBeInTheDocument());
    await waitFor(() => expect(screen.getByRole('button', { name: /Notifications, 2 unread/ })).toHaveFocus());
  });
});
