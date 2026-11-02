import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import AccountSection from '../account/AccountSection';
import { changePasswordErrorMessage } from '../account/ChangePasswordForm';
import { passwordProblem } from '../../auth/passwordPolicy';
import { httpError, plainUser, renderSettings } from './settingsTestUtils';

vi.mock('../../../services/api', async (importOriginal) => (await import('./apiMock')).mockApiModule(importOriginal));

const render = (changePassword = vi.fn().mockResolvedValue(undefined)) => {
  renderSettings(<AccountSection />, { path: '/settings/account', user: plainUser, auth: { changePassword } });
  return changePassword;
};

async function fill(user: ReturnType<typeof userEvent.setup>, current: string, next: string, confirm = next) {
  await user.type(screen.getByLabelText(/^Current password/), current);
  await user.type(screen.getByLabelText(/^New password/), next);
  await user.type(screen.getByLabelText(/^Confirm new password/), confirm);
}

describe('AccountSection', () => {
  it('shows who is signed in and the password form', () => {
    render();
    expect(screen.getByText('bob')).toBeInTheDocument();
    expect(screen.getByText('bob@example.com')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Change password' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Update password' })).toBeDisabled();
  });

  it('changes the password, clears the form and says other sessions were signed out', async () => {
    const user = userEvent.setup();
    const changePassword = render();
    await fill(user, 'old-password', 'new-password');
    await user.click(screen.getByRole('button', { name: 'Update password' }));
    await waitFor(() => expect(changePassword).toHaveBeenCalledWith('old-password', 'new-password'));
    expect(await screen.findByText(/Other sessions for your account have been signed out/)).toBeInTheDocument();
    expect(screen.getByLabelText(/^Current password/)).toHaveValue('');
  });

  it('enforces the password policy before calling the server', async () => {
    const user = userEvent.setup();
    const changePassword = render();
    await fill(user, 'old-password', 'short');
    await user.click(screen.getByRole('button', { name: 'Update password' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('at least 8 characters');
    expect(changePassword).not.toHaveBeenCalled();
  });

  it('asks for matching new passwords', async () => {
    const user = userEvent.setup();
    const changePassword = render();
    await fill(user, 'old-password', 'new-password', 'other-password');
    await user.click(screen.getByRole('button', { name: 'Update password' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('The new passwords do not match');
    expect(changePassword).not.toHaveBeenCalled();
  });

  it('explains a wrong current password', async () => {
    const user = userEvent.setup();
    render(vi.fn().mockRejectedValue(httpError(401)));
    await fill(user, 'wrong-password', 'new-password');
    await user.click(screen.getByRole('button', { name: 'Update password' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Current password is incorrect');
  });
});

describe('password change errors and policy', () => {
  const t = ((_key: string, fallback: string | { defaultValue: string }) =>
    typeof fallback === 'string' ? fallback : fallback.defaultValue) as never;

  it('maps each failure', () => {
    expect(changePasswordErrorMessage(httpError(429), t)).toBe('Too many attempts. Please try again later.');
    expect(changePasswordErrorMessage(httpError(400, { error: 'Password is too common' }), t)).toBe('Password is too common');
    expect(changePasswordErrorMessage(new Error('boom'), t)).toBe('Failed to change password');
  });

  it('matches the server policy on length', () => {
    expect(passwordProblem('1234567')).toBe('tooShort');
    expect(passwordProblem('12345678')).toBeNull();
    expect(passwordProblem('é'.repeat(37))).toBe('tooLong');
    expect(passwordProblem('a'.repeat(72))).toBeNull();
  });
});
