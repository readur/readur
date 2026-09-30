import { useTranslation } from 'react-i18next';
import { Switch } from '../../../ui';
import { isAdminRole } from '../../../auth/roles';
import { Tag } from '../shared/Facts';
import shared from '../shared/shared.module.css';
import type { UserRow } from './useUsers';

export interface UserStatusCellProps {
  user: Pick<UserRow, 'username' | 'role' | 'is_active'>;
  /** The signed-in administrator's own row: they cannot disable themselves. */
  isSelf: boolean;
  isDisabled?: boolean;
  onChange: (active: boolean) => void;
}

/** Account status (active, or disabled / awaiting approval) and the toggle that changes it. */
export function UserStatusCell({ user, isSelf, isDisabled = false, onChange }: UserStatusCellProps) {
  const { t } = useTranslation();
  const active = user.is_active !== false;

  return (
    <span className={shared.row}>
      <Switch
        aria-label={t('settings.userManagement.activeToggleLabel', {
          username: user.username,
          defaultValue: 'Account active for {{username}}',
        })}
        isSelected={active}
        onChange={onChange}
        isDisabled={isDisabled || isSelf}
      />
      <Tag tone={active ? 'ok' : 'default'}>
        {active
          ? t('settings.userManagement.statusActive', 'Active')
          : t('settings.userManagement.statusPending', 'Disabled / pending approval')}
      </Tag>
      {isAdminRole(user.role) ? <Tag>{t('settings.userManagement.roleAdmin', 'Admin')}</Tag> : null}
      {isSelf ? (
        <span className={shared.meta}>
          {t('settings.userManagement.cannotDeactivateSelf', 'You cannot deactivate your own account')}
        </span>
      ) : null}
    </span>
  );
}
