import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import RegisterRoute from '../RegisterRoute';
import { renderAuth, type RenderAuthOptions } from './authTestUtils';

vi.mock('../../../services/api', async () => {
  const errors = await import('../../../services/errors');
  return { ErrorHelper: errors.ErrorHelper, ErrorCodes: errors.ErrorCodes, api: { defaults: { headers: { common: {} } } } };
});

const renderRegister = (opts: Partial<RenderAuthOptions> = {}) =>
  renderAuth({ path: '/register', element: <RegisterRoute />, flags: { allowRegistration: true }, ...opts });

const where = () => screen.getByRole('status', { name: 'location' });

const fill = async (user: ReturnType<typeof userEvent.setup>, password = 'long-enough') => {
  await user.type(screen.getByRole('textbox', { name: /Username/ }), 'ada');
  await user.type(screen.getByRole('textbox', { name: /Email/ }), 'ada@example.com');
  await user.type(screen.getByLabelText(/^Password/), password);
};

const httpError = (status: number, data: unknown = {}) => ({ isAxiosError: true, response: { status, data } });

describe('RegisterRoute', () => {
  it('sends visitors to /login when registration is closed', async () => {
    renderRegister({ flags: { allowRegistration: false } });
    expect(await screen.findByRole('status', { name: 'location' })).toHaveTextContent('/login');
  });

  it('renders one h1 and a link back to sign in', () => {
    renderRegister();
    expect(screen.getByRole('heading', { level: 1, name: 'Create your Readur account' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Already have an account? Sign in' })).toHaveAttribute('href', '/login');
  });

  it('keeps the submit button off until every field is filled', async () => {
    const user = userEvent.setup();
    renderRegister();
    expect(screen.getByRole('button', { name: 'Sign up' })).toBeDisabled();
    await fill(user);
    expect(screen.getByRole('button', { name: 'Sign up' })).toBeEnabled();
  });

  it('rejects a short password without calling the server', async () => {
    const register = vi.fn();
    const user = userEvent.setup();
    renderRegister({ auth: { register } });
    await fill(user, 'short');
    await user.click(screen.getByRole('button', { name: 'Sign up' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('at least 8 characters');
    expect(register).not.toHaveBeenCalled();
  });

  it('explains that a new account awaits approval', async () => {
    const register = vi.fn().mockResolvedValue({ pendingApproval: true, user: {} });
    const user = userEvent.setup();
    renderRegister({ auth: { register } });
    await fill(user);
    await user.click(screen.getByRole('button', { name: 'Sign up' }));
    expect(await screen.findByText(/administrator must approve it/)).toBeInTheDocument();
    expect(register).toHaveBeenCalledWith('ada', 'ada@example.com', 'long-enough');
    expect(screen.getByRole('link', { name: 'Back to sign in' })).toHaveAttribute('href', '/login');
  });

  it('goes home when the account is active straight away', async () => {
    const register = vi.fn().mockResolvedValue({ pendingApproval: false, user: {} });
    const user = userEvent.setup();
    renderRegister({ auth: { register } });
    await fill(user);
    await user.click(screen.getByRole('button', { name: 'Sign up' }));
    expect(await screen.findByRole('status', { name: 'location' })).toHaveTextContent('/home');
    expect(where()).toBeInTheDocument();
  });

  it.each([
    [403, 'Self-registration is disabled on this server.'],
    [429, 'Too many registration attempts'],
  ])('maps a %s response to a fixed message', async (status, message) => {
    const register = vi.fn().mockRejectedValue(httpError(status, { error: 'server text' }));
    const user = userEvent.setup();
    renderRegister({ auth: { register } });
    await fill(user);
    await user.click(screen.getByRole('button', { name: 'Sign up' }));
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent(message);
    expect(alert).not.toHaveTextContent('server text');
  });

  it('shows why the server refused other registrations', async () => {
    const register = vi.fn().mockRejectedValue(httpError(400, { error: 'Username already exists' }));
    const user = userEvent.setup();
    renderRegister({ auth: { register } });
    await fill(user);
    await user.click(screen.getByRole('button', { name: 'Sign up' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Username already exists');
  });
});
