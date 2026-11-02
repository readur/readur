import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { userWatchService } from '../../../services/api';
import UsersSection from '../users/UsersSection';
import { adminUser, apiMock, httpError, ok, renderSettings } from './settingsTestUtils';

vi.mock('../../../services/api', async (importOriginal) => (await import('./apiMock')).mockApiModule(importOriginal));

const USERS = [
  { id: adminUser.id, username: 'ada', email: 'ada@example.com', role: 'admin', created_at: '2026-01-02T00:00:00Z' },
  { id: 'u2', username: 'bob', email: 'bob@example.com', role: 'user', created_at: '2026-03-04T00:00:00Z' },
];

const render = (perUserWatch = false) => renderSettings(<UsersSection />, { path: '/settings/users', perUserWatch });

beforeEach(() => {
  apiMock.get.mockResolvedValue(ok(USERS));
  apiMock.post.mockResolvedValue(ok({}));
  apiMock.put.mockResolvedValue(ok({}));
  apiMock.delete.mockResolvedValue(ok(null));
});

describe('UsersSection', () => {
  it('lists users in a table', async () => {
    render();
    const grid = await screen.findByRole('grid', { name: 'User Management' });
    expect(within(grid).getByRole('row', { name: /bob/ })).toHaveTextContent('bob@example.com');
    expect(apiMock.get).toHaveBeenCalledWith('/users');
  });

  it('hides the created-at column when no user has a date', async () => {
    apiMock.get.mockResolvedValue(ok(USERS.map((u) => ({ ...u, created_at: undefined }))));
    render();
    const grid = await screen.findByRole('grid', { name: 'User Management' });
    await within(grid).findByText('bob');
    expect(within(grid).queryByRole('columnheader', { name: /created/i })).not.toBeInTheDocument();
  });

  it('validates the create form inline', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Add user' }));
    const dialog = await screen.findByRole('dialog', { name: 'Create New User' });
    await user.type(within(dialog).getByRole('textbox', { name: /Email/ }), 'not-an-email');
    await user.click(within(dialog).getByRole('button', { name: 'Create' }));
    expect(within(dialog).getByText('Enter a username.')).toBeInTheDocument();
    expect(within(dialog).getByText('Please enter a valid email address.')).toBeInTheDocument();
    expect(within(dialog).getByText('Enter a password.')).toBeInTheDocument();
    expect(apiMock.post).not.toHaveBeenCalled();
  });

  it('creates a user and toasts', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Add user' }));
    const dialog = await screen.findByRole('dialog', { name: 'Create New User' });
    await user.type(within(dialog).getByRole('textbox', { name: /Username/ }), 'carol');
    await user.type(within(dialog).getByRole('textbox', { name: /Email/ }), 'carol@example.com');
    await user.type(within(dialog).getByLabelText(/^Password/), 's3cretPass');
    await user.click(within(dialog).getByRole('button', { name: 'Create' }));
    await waitFor(() =>
      expect(apiMock.post).toHaveBeenCalledWith('/users', {
        username: 'carol',
        email: 'carol@example.com',
        password: 's3cretPass',
      }),
    );
    expect(await screen.findByText('User created successfully')).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });

  it('edits a user without sending an empty password', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Edit bob' }));
    const dialog = await screen.findByRole('dialog', { name: 'Edit User' });
    const email = within(dialog).getByRole('textbox', { name: /Email/ });
    expect(email).toHaveValue('bob@example.com');
    await user.clear(email);
    await user.type(email, 'robert@example.com');
    await user.click(within(dialog).getByRole('button', { name: 'Update' }));
    await waitFor(() =>
      expect(apiMock.put).toHaveBeenCalledWith('/users/u2', { username: 'bob', email: 'robert@example.com' }),
    );
    expect(await screen.findByText('User updated successfully')).toBeInTheDocument();
  });

  it('sends a new password when one is typed', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Edit bob' }));
    const dialog = await screen.findByRole('dialog', { name: 'Edit User' });
    await user.type(within(dialog).getByLabelText(/New Password/), 'n3wPassword');
    await user.click(within(dialog).getByRole('button', { name: 'Update' }));
    await waitFor(() =>
      expect(apiMock.put).toHaveBeenCalledWith('/users/u2', { username: 'bob', email: 'bob@example.com', password: 'n3wPassword' }),
    );
  });

  it('keeps the dialog open with the mapped server error', async () => {
    apiMock.post.mockRejectedValue(httpError(409, { error: 'dup', code: 'USER_DUPLICATE_USERNAME' }));
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Add user' }));
    const dialog = await screen.findByRole('dialog', { name: 'Create New User' });
    await user.type(within(dialog).getByRole('textbox', { name: /Username/ }), 'bob');
    await user.type(within(dialog).getByRole('textbox', { name: /Email/ }), 'b@example.com');
    await user.type(within(dialog).getByLabelText(/^Password/), 'x');
    await user.click(within(dialog).getByRole('button', { name: 'Create' }));
    expect(
      await within(dialog).findByText('This username is already taken. Please choose a different username.'),
    ).toBeInTheDocument();
  });

  it('confirms before deleting and deletes on confirm', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Delete bob' }));
    const confirm = await screen.findByRole('alertdialog', { name: 'Delete user' });
    expect(within(confirm).getByText('Are you sure you want to delete bob (bob@example.com)?')).toBeInTheDocument();
    expect(apiMock.delete).not.toHaveBeenCalled();
    await user.click(within(confirm).getByRole('button', { name: 'Delete' }));
    await waitFor(() => expect(apiMock.delete).toHaveBeenCalledWith('/users/u2'));
    expect(await screen.findByText('User deleted successfully')).toBeInTheDocument();
  });

  it('does not delete when the confirmation is cancelled', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Delete bob' }));
    await user.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Cancel' }));
    await waitFor(() => expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument());
    expect(apiMock.delete).not.toHaveBeenCalled();
  });

  it('never lets you delete yourself', async () => {
    render();
    const user = userEvent.setup();
    const del = await screen.findByRole('button', { name: 'Delete ada' });
    expect(del).toHaveAttribute('aria-disabled', 'true');
    await user.click(del);
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
  });

  it('explains a restricted delete', async () => {
    apiMock.delete.mockRejectedValue(httpError(409, { error: 'x', code: 'USER_DELETE_RESTRICTED' }));
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Delete bob' }));
    await user.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Delete' }));
    expect(
      await screen.findByText('Cannot delete this user: They may have associated data or be the last admin.'),
    ).toBeInTheDocument();
  });

  it('shows per-user watch directories when enabled and creates one', async () => {
    vi.spyOn(userWatchService, 'getUserWatchDirectory').mockImplementation(async (id: string) => {
      if (id === 'u2') throw httpError(404);
      return { data: { user_id: id, username: 'ada', watch_directory_path: './user_watch/ada', exists: true, enabled: true } } as never;
    });
    vi.spyOn(userWatchService, 'createUserWatchDirectory').mockResolvedValue({
      data: { success: true, message: 'ok', watch_directory_path: './user_watch/bob' },
    } as never);
    const user = userEvent.setup();
    render(true);
    const grid = await screen.findByRole('grid', { name: 'User Management' });
    expect(await within(grid).findByText('./user_watch/ada')).toBeInTheDocument();
    // Account rows no longer repeat "Active"; only ada's watch directory says it.
    expect(within(grid).getAllByText('Active')).toHaveLength(1);
    expect(within(grid).getByText('Not Created')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Create watch directory for bob' }));
    expect(userWatchService.createUserWatchDirectory).toHaveBeenCalledWith('u2');
    expect(await screen.findByText('Watch directory created successfully')).toBeInTheDocument();
  });

  it('confirms before removing a watch directory', async () => {
    vi.spyOn(userWatchService, 'getUserWatchDirectory').mockResolvedValue({
      data: { user_id: 'u2', username: 'bob', watch_directory_path: './user_watch/bob', exists: true, enabled: true },
    } as never);
    vi.spyOn(userWatchService, 'deleteUserWatchDirectory').mockResolvedValue({
      data: { success: true, message: 'ok', watch_directory_path: null },
    } as never);
    const user = userEvent.setup();
    render(true);
    await user.click(await screen.findByRole('button', { name: 'Remove watch directory for bob' }));
    const confirm = await screen.findByRole('alertdialog', { name: 'Remove Watch Directory' });
    expect(userWatchService.deleteUserWatchDirectory).not.toHaveBeenCalled();
    await user.click(within(confirm).getByRole('button', { name: 'Remove Directory' }));
    await waitFor(() => expect(userWatchService.deleteUserWatchDirectory).toHaveBeenCalledWith('u2'));
    expect(await screen.findByText('Watch directory removed successfully')).toBeInTheDocument();
  });

  it('hides watch directory controls when the feature is off', async () => {
    render(false);
    await screen.findByRole('grid', { name: 'User Management' });
    expect(screen.queryByRole('columnheader', { name: 'Watch Directory' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /watch directory/i })).not.toBeInTheDocument();
  });

  it('shows each account status and never lets an admin disable themselves', async () => {
    render();
    const grid = await screen.findByRole('grid', { name: 'User Management' });
    expect(within(grid).getByRole('switch', { name: 'Account active for bob' })).toBeChecked();
    expect(within(grid).getByRole('switch', { name: 'Account active for ada' })).toBeDisabled();
    // The reason is on the muted delete button's tooltip, not a line that unbalances the row.
    expect(within(grid).queryByText(/cannot deactivate your own account/)).not.toBeInTheDocument();
    const del = within(grid).getAllByRole('button').find((b) => b.getAttribute('aria-disabled') === 'true') as HTMLElement;
    expect(del).toBeDefined();
    const user = userEvent.setup();
    await user.click(document.body); // a pointer interaction first, as React Aria tooltips expect
    await user.hover(del);
    expect(await screen.findByRole('tooltip')).toHaveTextContent(/deactivate or delete your own account/);
  });

  it('flags accounts that are disabled or awaiting approval', async () => {
    apiMock.get.mockResolvedValue(ok([USERS[0], { ...USERS[1], is_active: false }]));
    render();
    const grid = await screen.findByRole('grid', { name: 'User Management' });
    expect(within(grid).getByText('Disabled / pending approval')).toBeInTheDocument();
    expect(within(grid).getByRole('switch', { name: 'Account active for bob' })).not.toBeChecked();
    expect(screen.getByText(/1 account\(s\) are disabled or awaiting approval/)).toBeInTheDocument();
  });

  it('enables and disables an account', async () => {
    const user = userEvent.setup();
    render();
    const grid = await screen.findByRole('grid', { name: 'User Management' });
    await user.click(within(grid).getByRole('switch', { name: 'Account active for bob' }));
    await waitFor(() => expect(apiMock.put).toHaveBeenCalledWith('/users/u2', { is_active: false }));
    expect(await screen.findByText('bob has been disabled and signed out')).toBeInTheDocument();
    expect(within(grid).getByRole('switch', { name: 'Account active for bob' })).not.toBeChecked();
    await user.click(within(grid).getByRole('switch', { name: 'Account active for bob' }));
    await waitFor(() => expect(apiMock.put).toHaveBeenCalledWith('/users/u2', { is_active: true }));
    expect(await screen.findByText('bob can now sign in')).toBeInTheDocument();
  });

  it('keeps the status when the change fails', async () => {
    apiMock.put.mockRejectedValue(httpError(500));
    const user = userEvent.setup();
    render();
    const grid = await screen.findByRole('grid', { name: 'User Management' });
    await user.click(within(grid).getByRole('switch', { name: 'Account active for bob' }));
    await waitFor(() => expect(apiMock.put).toHaveBeenCalled());
    expect(within(grid).getByRole('switch', { name: 'Account active for bob' })).toBeChecked();
  });

  it('shows an error when users fail to load', async () => {
    apiMock.get.mockRejectedValue(httpError(500));
    render();
    expect(await screen.findByRole('alert')).toHaveTextContent('Failed to load users');
  });
});
