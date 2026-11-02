import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { apiKeysService, type ApiKey } from '../../../services/api';
import ApiKeysSection from '../apiKeys/ApiKeysSection';
import { httpError, renderSettings } from './settingsTestUtils';

vi.mock('../../../services/api', async (importOriginal) => (await import('./apiMock')).mockApiModule(importOriginal));

const key = (over: Partial<ApiKey>): ApiKey => ({
  id: 'k1',
  user_id: 'u-admin',
  name: 'backup-script',
  key_prefix: 'readur_pat_ab12',
  expires_at: '2026-12-01T00:00:00Z',
  last_used_at: null,
  revoked_at: null,
  is_expired: false,
  created_at: '2026-09-01T00:00:00Z',
  ...over,
});

const KEYS = [
  key({}),
  key({ id: 'k2', name: 'old-sync', revoked_at: '2026-09-10T00:00:00Z' }),
  key({ id: 'k3', name: 'expired-one', is_expired: true }),
];
const PLAINTEXT = 'readur_pat_ab12cd34ef56gh78ij90';


beforeEach(() => {
  vi.spyOn(apiKeysService, 'list').mockResolvedValue({ data: KEYS } as never);
  vi.spyOn(apiKeysService, 'create').mockResolvedValue({
    data: { api_key: key({ id: 'k9', name: 'ci' }), plaintext: PLAINTEXT },
  } as never);
  vi.spyOn(apiKeysService, 'revoke').mockResolvedValue({ data: null } as never);
});

const render = () => renderSettings(<ApiKeysSection />, { path: '/settings/api-keys' });

async function createKey(user: ReturnType<typeof userEvent.setup>, name = 'ci') {
  await user.click(await screen.findByRole('button', { name: 'Create API key' }));
  const dialog = await screen.findByRole('dialog', { name: 'Create API key' });
  await user.type(within(dialog).getByRole('textbox', { name: /Name/ }), name);
  await user.click(within(dialog).getByRole('button', { name: 'Create' }));
}

describe('ApiKeysSection', () => {
  it('lists keys with prefix and a status word', async () => {
    render();
    const table = await screen.findByRole('grid', { name: 'API keys' });
    const row = within(table).getByRole('row', { name: /backup-script/ });
    expect(within(row).getByText('readur_pat_ab12…')).toBeInTheDocument();
    expect(within(row).getByText('Active')).toBeInTheDocument();
    expect(within(within(table).getByRole('row', { name: /old-sync/ })).getByText('Revoked')).toBeInTheDocument();
    expect(within(within(table).getByRole('row', { name: /expired-one/ })).getByText('Expired')).toBeInTheDocument();
  });

  it('shows an empty state', async () => {
    vi.mocked(apiKeysService.list).mockResolvedValue({ data: [] } as never);
    render();
    expect(await screen.findByText("You don't have any API keys yet.")).toBeInTheDocument();
  });

  it('shows a load error from the server', async () => {
    vi.mocked(apiKeysService.list).mockRejectedValue(httpError(500, { error: 'database down' }));
    render();
    expect(await screen.findByRole('alert')).toHaveTextContent('database down');
  });

  it('requires a name before creating', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Create API key' }));
    const dialog = await screen.findByRole('dialog', { name: 'Create API key' });
    await user.click(within(dialog).getByRole('button', { name: 'Create' }));
    expect(within(dialog).getByText('Please give this key a name.')).toBeInTheDocument();
    expect(apiKeysService.create).not.toHaveBeenCalled();
  });

  it('creates with the chosen expiry and shows the key once', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Create API key' }));
    const dialog = await screen.findByRole('dialog', { name: 'Create API key' });
    await user.type(within(dialog).getByRole('textbox', { name: /Name/ }), '  ci  ');
    await user.click(within(dialog).getByRole('button', { name: /Expiration/ }));
    await user.click(await screen.findByRole('option', { name: '30 days' }));
    await user.click(within(dialog).getByRole('button', { name: 'Create' }));
    expect(apiKeysService.create).toHaveBeenCalledWith({ name: 'ci', expires_in_days: 30 });
    const reveal = await screen.findByRole('dialog', { name: 'Your new API key' });
    expect(within(reveal).getByText(PLAINTEXT)).toBeInTheDocument();
    expect(within(reveal).getByText(/it will not be shown again/)).toBeInTheDocument();
    await user.click(within(reveal).getByRole('button', { name: "I've saved it" }));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(screen.queryByText(PLAINTEXT)).not.toBeInTheDocument();
    expect(apiKeysService.list).toHaveBeenCalledTimes(2);
  });

  it('sends no expiry for "Never"', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Create API key' }));
    const dialog = await screen.findByRole('dialog', { name: 'Create API key' });
    await user.type(within(dialog).getByRole('textbox', { name: /Name/ }), 'forever');
    await user.click(within(dialog).getByRole('button', { name: /Expiration/ }));
    await user.click(await screen.findByRole('option', { name: 'Never (not recommended)' }));
    await user.click(within(dialog).getByRole('button', { name: 'Create' }));
    expect(apiKeysService.create).toHaveBeenCalledWith({ name: 'forever', expires_in_days: undefined });
  });

  it('keeps the new key open on Escape: only the explicit button closes it', async () => {
    const user = userEvent.setup();
    render();
    await createKey(user);
    await screen.findByRole('dialog', { name: 'Your new API key' });
    await user.keyboard('{Escape}');
    expect(screen.getByRole('dialog', { name: 'Your new API key' })).toBeInTheDocument();
  });

  it('copies the key to the clipboard', async () => {
    const user = userEvent.setup();
    render();
    await createKey(user);
    const reveal = await screen.findByRole('dialog', { name: 'Your new API key' });
    // user-event installs its own clipboard stub on setup; read back what landed there.
    await user.click(within(reveal).getByRole('button', { name: 'Copy API key' }));
    expect(await navigator.clipboard.readText()).toBe(PLAINTEXT);
    expect(await within(reveal).findByRole('button', { name: 'Copied' })).toBeInTheDocument();
  });

  it('shows the server error inside the create dialog', async () => {
    vi.mocked(apiKeysService.create).mockRejectedValue(httpError(400, { error: 'name taken' }));
    const user = userEvent.setup();
    render();
    await createKey(user);
    const dialog = screen.getByRole('dialog', { name: 'Create API key' });
    expect(await within(dialog).findByText('name taken')).toBeInTheDocument();
    expect(screen.queryByRole('dialog', { name: 'Your new API key' })).not.toBeInTheDocument();
  });

  it('confirms before revoking and revokes on confirm', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Revoke backup-script' }));
    const confirm = await screen.findByRole('alertdialog', { name: 'Revoke API key' });
    expect(within(confirm).getByText(/will immediately stop working/)).toBeInTheDocument();
    expect(apiKeysService.revoke).not.toHaveBeenCalled();
    await user.click(within(confirm).getByRole('button', { name: 'Revoke' }));
    await waitFor(() => expect(apiKeysService.revoke).toHaveBeenCalledWith('k1'));
    await waitFor(() => expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument());
  });

  it('does not revoke when the confirmation is cancelled', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Revoke backup-script' }));
    await user.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Cancel' }));
    await waitFor(() => expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument());
    expect(apiKeysService.revoke).not.toHaveBeenCalled();
  });

  it('shows a revoke failure inside the confirmation', async () => {
    vi.mocked(apiKeysService.revoke).mockRejectedValue(httpError(500, { error: 'nope' }));
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Revoke backup-script' }));
    const confirm = await screen.findByRole('alertdialog');
    await user.click(within(confirm).getByRole('button', { name: 'Revoke' }));
    expect(await within(confirm).findByText('nope')).toBeInTheDocument();
  });

  it('disables revoking an already revoked key', async () => {
    render();
    expect(await screen.findByRole('button', { name: 'Already revoked' })).toBeDisabled();
  });
});
