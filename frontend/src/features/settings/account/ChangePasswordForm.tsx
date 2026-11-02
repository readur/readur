import { useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';
import { useAuth } from '../../../contexts/AuthContext';
import { Button, TextField } from '../../../ui';
import { MIN_PASSWORD_LENGTH, validateNewPassword } from '../../auth/passwordPolicy';
import { Notice } from '../shared/Notice';
import shared from '../shared/shared.module.css';
import formStyles from '../form/form.module.css';

/** Maps a failed password change to a message. */
export function changePasswordErrorMessage(err: unknown, t: TFunction): string {
  const response = (err as { response?: { status?: number; data?: { error?: unknown } } } | null)?.response;
  if (response?.status === 401) return t('settings.account.wrongCurrentPassword', 'Current password is incorrect');
  if (response?.status === 429) return t('settings.account.tooManyAttempts', 'Too many attempts. Please try again later.');
  const serverMessage = response?.data?.error;
  return typeof serverMessage === 'string' && serverMessage
    ? serverMessage
    : t('settings.account.passwordChangeFailed', 'Failed to change password');
}

/**
 * Change the signed-in user's password. The server signs out every other session and hands back
 * a new one for this browser, so the user stays signed in here.
 */
export function ChangePasswordForm() {
  const { t } = useTranslation();
  const { changePassword } = useAuth();
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');
  const [saving, setSaving] = useState(false);

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    if (saving) return;
    setError('');
    setSuccess('');
    const policyError = validateNewPassword(newPassword, t);
    if (policyError) {
      setError(policyError);
      return;
    }
    if (newPassword !== confirmPassword) {
      setError(t('settings.account.passwordMismatch', 'The new passwords do not match'));
      return;
    }
    setSaving(true);
    try {
      await changePassword(currentPassword, newPassword);
      setCurrentPassword('');
      setNewPassword('');
      setConfirmPassword('');
      setSuccess(
        t('settings.account.passwordChanged', 'Password changed. Other sessions for your account have been signed out.'),
      );
    } catch (err) {
      setError(changePasswordErrorMessage(err, t));
    } finally {
      setSaving(false);
    }
  };

  return (
    <section className={shared.panel} aria-labelledby="change-password-title">
      <h3 id="change-password-title" className={shared.subheading}>
        {t('settings.account.changePassword', 'Change password')}
      </h3>
      <form className={formStyles.form} onSubmit={onSubmit} noValidate>
        {error ? <Notice tone="danger">{error}</Notice> : null}
        {success ? <Notice>{success}</Notice> : null}
        <div className={formStyles.fields}>
          <TextField
            label={t('settings.account.currentPassword', 'Current password')}
            type="password"
            autoComplete="current-password"
            isRequired
            value={currentPassword}
            onChange={setCurrentPassword}
          />
          <TextField
            label={t('settings.account.newPassword', 'New password')}
            type="password"
            autoComplete="new-password"
            isRequired
            value={newPassword}
            onChange={setNewPassword}
            description={t('settings.account.passwordPolicy', {
              min: MIN_PASSWORD_LENGTH,
              defaultValue: 'At least {{min}} characters',
            })}
          />
          <TextField
            label={t('settings.account.confirmNewPassword', 'Confirm new password')}
            type="password"
            autoComplete="new-password"
            isRequired
            value={confirmPassword}
            onChange={setConfirmPassword}
          />
        </div>
        <div className={formStyles.actions}>
          <Button
            type="submit"
            variant="primary"
            isPending={saving}
            isDisabled={!currentPassword || !newPassword || !confirmPassword}
          >
            {t('settings.account.updatePassword', 'Update password')}
          </Button>
        </div>
      </form>
    </section>
  );
}
