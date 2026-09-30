import { useEffect, useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Dialog, TextField } from '../../../ui';
import { Notice } from '../shared/Notice';
import formStyles from '../form/form.module.css';
import type { UserForm, UserRow } from './useUsers';

export interface UserDialogProps {
  isOpen: boolean;
  /** The user being edited, or null to create one. */
  user: UserRow | null;
  onClose: () => void;
  onSubmit: (form: UserForm, existing: UserRow | null) => Promise<void>;
}

const EMPTY: UserForm = { username: '', email: '', password: '' };
const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

/** Create or edit a user. Username and email are required; the password only when creating. */
export function UserDialog({ isOpen, user, onClose, onSubmit }: UserDialogProps) {
  const { t } = useTranslation();
  const [form, setForm] = useState<UserForm>(EMPTY);
  const [errors, setErrors] = useState<Partial<Record<keyof UserForm, string>>>({});
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const creating = user === null;

  useEffect(() => {
    if (!isOpen) return;
    setForm(user ? { username: user.username, email: user.email, password: '' } : EMPTY);
    setErrors({});
    setSubmitError(null);
  }, [isOpen, user]);

  const validate = (): boolean => {
    const next: Partial<Record<keyof UserForm, string>> = {};
    if (!form.username.trim()) next.username = t('settings.users.usernameRequired', 'Enter a username.');
    if (!form.email.trim()) next.email = t('settings.users.emailRequired', 'Enter an email address.');
    else if (!EMAIL.test(form.email.trim())) next.email = t('settings.messages.invalidEmail');
    if (creating && !form.password) next.password = t('settings.users.passwordRequired', 'Enter a password.');
    setErrors(next);
    return Object.keys(next).length === 0;
  };

  const submit = async (e?: FormEvent) => {
    e?.preventDefault();
    if (!validate()) return;
    setSaving(true);
    setSubmitError(null);
    try {
      await onSubmit({ ...form, username: form.username.trim(), email: form.email.trim() }, user);
      onClose();
    } catch (err) {
      setSubmitError(err instanceof Error ? err.message : t('settings.messages.settingsUpdateFailed'));
    } finally {
      setSaving(false);
    }
  };

  const set = (key: keyof UserForm) => (value: string) => setForm((f) => ({ ...f, [key]: value }));

  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={(open) => !open && !saving && onClose()}
      title={creating ? t('settings.userManagement.dialogs.createUser') : t('settings.userManagement.dialogs.editUser')}
      actions={
        <>
          <Button variant="ghost" onPress={onClose} isDisabled={saving}>
            {t('common.actions.cancel')}
          </Button>
          <Button variant="primary" onPress={() => void submit()} isPending={saving}>
            {creating ? t('common.actions.create') : t('common.actions.update')}
          </Button>
        </>
      }
    >
      <form className={formStyles.form} onSubmit={submit} noValidate>
        <TextField
          label={t('settings.userManagement.dialogs.username')}
          value={form.username}
          onChange={set('username')}
          isRequired
          isInvalid={Boolean(errors.username)}
          errorMessage={errors.username}
          validationBehavior="aria"
          autoFocus
        />
        <TextField
          label={t('settings.userManagement.dialogs.email')}
          type="email"
          value={form.email}
          onChange={set('email')}
          isRequired
          isInvalid={Boolean(errors.email)}
          errorMessage={errors.email}
          validationBehavior="aria"
        />
        <TextField
          label={creating ? t('settings.userManagement.dialogs.password') : t('settings.userManagement.dialogs.newPassword')}
          type="password"
          value={form.password}
          onChange={set('password')}
          isRequired={creating}
          isInvalid={Boolean(errors.password)}
          errorMessage={errors.password}
          validationBehavior="aria"
        />
        {submitError ? <Notice tone="danger">{submitError}</Notice> : null}
      </form>
    </Dialog>
  );
}
