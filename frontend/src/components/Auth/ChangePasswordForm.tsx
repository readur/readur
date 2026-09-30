import React, { useState } from 'react';
import { Alert, Box, Button, Card, CardContent, CircularProgress, Divider, TextField, Typography } from '@mui/material';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../../contexts/AuthContext';

export const MIN_PASSWORD_LENGTH = 8;
export const MAX_PASSWORD_BYTES = 72;

/** Returns a validation message for a new password, or null when it satisfies the server policy. */
export const validateNewPassword = (password: string): string | null => {
  if (password.length < MIN_PASSWORD_LENGTH) {
    return `Password must be at least ${MIN_PASSWORD_LENGTH} characters long`;
  }
  if (new TextEncoder().encode(password).length > MAX_PASSWORD_BYTES) {
    return `Password must be at most ${MAX_PASSWORD_BYTES} bytes long`;
  }
  return null;
};

const ChangePasswordForm: React.FC = () => {
  const { t } = useTranslation();
  const { changePassword } = useAuth();
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');
  const [saving, setSaving] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setSuccess('');

    const policyError = validateNewPassword(newPassword);
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
        t(
          'settings.account.passwordChanged',
          'Password changed. Other sessions for your account have been signed out.'
        )
      );
    } catch (err: any) {
      const status = err?.response?.status;
      const serverMessage = err?.response?.data?.error;
      if (status === 401) {
        setError(t('settings.account.wrongCurrentPassword', 'Current password is incorrect'));
      } else if (status === 429) {
        setError(t('settings.account.tooManyAttempts', 'Too many attempts. Please try again later.'));
      } else if (typeof serverMessage === 'string' && serverMessage) {
        setError(serverMessage);
      } else {
        setError(t('settings.account.passwordChangeFailed', 'Failed to change password'));
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <Card sx={{ mb: 3 }}>
      <CardContent>
        <Typography variant="subtitle1" sx={{ mb: 2 }}>
          {t('settings.account.changePassword', 'Change password')}
        </Typography>
        <Divider sx={{ mb: 2 }} />
        <Box component="form" onSubmit={handleSubmit} noValidate sx={{ maxWidth: 420 }}>
          {error && (
            <Alert severity="error" sx={{ mb: 2 }}>
              {error}
            </Alert>
          )}
          {success && (
            <Alert severity="success" sx={{ mb: 2 }}>
              {success}
            </Alert>
          )}
          <TextField
            fullWidth
            margin="normal"
            type="password"
            label={t('settings.account.currentPassword', 'Current password')}
            autoComplete="current-password"
            value={currentPassword}
            onChange={(e) => setCurrentPassword(e.target.value)}
            required
          />
          <TextField
            fullWidth
            margin="normal"
            type="password"
            label={t('settings.account.newPassword', 'New password')}
            autoComplete="new-password"
            value={newPassword}
            onChange={(e) => setNewPassword(e.target.value)}
            helperText={t('settings.account.passwordPolicy', {
              min: MIN_PASSWORD_LENGTH,
              defaultValue: 'At least {{min}} characters',
            })}
            required
          />
          <TextField
            fullWidth
            margin="normal"
            type="password"
            label={t('settings.account.confirmNewPassword', 'Confirm new password')}
            autoComplete="new-password"
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
            required
          />
          <Button
            type="submit"
            variant="contained"
            sx={{ mt: 2 }}
            disabled={saving || !currentPassword || !newPassword || !confirmPassword}
            startIcon={saving ? <CircularProgress size={16} /> : undefined}
          >
            {t('settings.account.updatePassword', 'Update password')}
          </Button>
        </Box>
      </CardContent>
    </Card>
  );
};

export default ChangePasswordForm;
