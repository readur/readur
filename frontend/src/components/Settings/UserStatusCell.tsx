import React from 'react';
import { Box, Chip, Switch, Tooltip } from '@mui/material';
import { useTranslation } from 'react-i18next';

interface UserStatusCellProps {
  user: { username: string; role?: string; is_active?: boolean };
  /** The signed-in administrator's own row: they cannot deactivate themselves. */
  isSelf: boolean;
  disabled?: boolean;
  onChange: (active: boolean) => void;
}

/** Account status and enable/disable toggle for the user management table. */
const UserStatusCell: React.FC<UserStatusCellProps> = ({ user, isSelf, disabled = false, onChange }) => {
  const { t } = useTranslation();
  const active = user.is_active !== false;

  const tooltip = isSelf
    ? t('settings.userManagement.cannotDeactivateSelf', 'You cannot deactivate your own account')
    : active
      ? t('settings.userManagement.disableUser', 'Disable account')
      : t('settings.userManagement.enableUser', 'Enable account');

  return (
    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
      <Tooltip title={tooltip}>
        <span>
          <Switch
            size="small"
            checked={active}
            onChange={(e) => onChange(e.target.checked)}
            disabled={disabled || isSelf}
            inputProps={{
              'aria-label': t('settings.userManagement.activeToggleLabel', {
                username: user.username,
                defaultValue: 'Account active for {{username}}',
              }),
            }}
          />
        </span>
      </Tooltip>
      {active ? (
        <Chip size="small" color="success" variant="outlined" label={t('settings.userManagement.statusActive', 'Active')} />
      ) : (
        <Chip size="small" color="warning" label={t('settings.userManagement.statusPending', 'Disabled / pending approval')} />
      )}
      {user.role === 'admin' && (
        <Chip size="small" variant="outlined" label={t('settings.userManagement.roleAdmin', 'Admin')} />
      )}
    </Box>
  );
};

export default UserStatusCell;
