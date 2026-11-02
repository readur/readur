import type { TFunction } from 'i18next';

/** The server's password policy: at least 8 characters, at most 72 bytes (bcrypt's input limit). */
export const MIN_PASSWORD_LENGTH = 8;
export const MAX_PASSWORD_BYTES = 72;

export type PasswordProblem = 'tooShort' | 'tooLong';

/** Why a new password breaks the server policy, or null when it satisfies it. */
export function passwordProblem(password: string): PasswordProblem | null {
  if (password.length < MIN_PASSWORD_LENGTH) return 'tooShort';
  if (new TextEncoder().encode(password).length > MAX_PASSWORD_BYTES) return 'tooLong';
  return null;
}

/** A validation message for a new password, or null when it satisfies the server policy. */
export function validateNewPassword(password: string, t: TFunction): string | null {
  switch (passwordProblem(password)) {
    case 'tooShort':
      return t('register.errors.passwordTooShort', {
        min: MIN_PASSWORD_LENGTH,
        defaultValue: 'Password must be at least {{min}} characters long',
      });
    case 'tooLong':
      return t('settings.account.passwordTooLong', {
        max: MAX_PASSWORD_BYTES,
        defaultValue: 'Password must be at most {{max}} bytes long',
      });
    default:
      return null;
  }
}
