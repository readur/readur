import { Button as RACButton, Header, MenuSection, Separator, type Key } from 'react-aria-components';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Avatar, IconButton, Menu, MenuItem, MenuTrigger } from '../../ui';
import { AccountCircle, Api, Logout, Settings } from '../../ui/icons';
import { useAuth } from '../../contexts/AuthContext';
import styles from './AppShell.module.css';

export const API_DOCS_PATH = '/swagger-ui';

export interface UserMenuProps {
  /** `row` shows the avatar and username (sidebar foot); `icon` is the bare button (phone top bar). */
  variant?: 'row' | 'icon';
}

export function UserMenu({ variant = 'row' }: UserMenuProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { user, logout } = useAuth();

  const onAction = (key: Key) => {
    if (key === 'settings') navigate('/settings');
    if (key === 'api-docs') window.open(API_DOCS_PATH, '_blank', 'noopener,noreferrer');
    if (key === 'logout') {
      void logout();
      navigate('/login');
    }
  };

  return (
    <MenuTrigger>
      {variant === 'row' && user ? (
        <RACButton
          className={styles.userRow}
          aria-label={t('shell.user.menuFor', { name: user.username, defaultValue: 'Account menu, {{name}}' })}
        >
          <Avatar name={user.username} size="sm" />
          <span className={styles.userRowText}>
            <span className={styles.userRowName}>{user.username}</span>
            {user.role ? <span className={styles.userRowRole}>{t(`shell.user.role.${user.role}`, user.role)}</span> : null}
          </span>
        </RACButton>
      ) : (
        <IconButton label={t('shell.user.menu', 'Account menu')} icon={<AccountCircle fontSize="inherit" />} />
      )}
      <Menu aria-label={t('shell.user.menu', 'Account menu')} onAction={onAction}>
        {user ? (
          <MenuSection className={styles.userSection}>
            <Header className={styles.userHeader}>
              <span className={styles.userName}>{user.username}</span>
              <span className={styles.userEmail}>{user.email}</span>
            </Header>
          </MenuSection>
        ) : null}
        <MenuSection>
          <MenuItem id="settings" textValue={t('shell.user.settings', 'Settings')}>
            <Settings fontSize="inherit" aria-hidden="true" />
            {t('shell.user.settings', 'Settings')}
          </MenuItem>
          <MenuItem id="api-docs" textValue={t('shell.user.apiDocs', 'API documentation')}>
            <Api fontSize="inherit" aria-hidden="true" />
            {t('shell.user.apiDocs', 'API documentation')}
          </MenuItem>
        </MenuSection>
        <Separator className={styles.menuSeparator} />
        <MenuSection>
          <MenuItem id="logout" textValue={t('shell.user.logout', 'Log out')}>
            <Logout fontSize="inherit" aria-hidden="true" />
            {t('shell.user.logout', 'Log out')}
          </MenuItem>
        </MenuSection>
      </Menu>
    </MenuTrigger>
  );
}
