import { Header, MenuSection, Separator, type Key } from 'react-aria-components';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { IconButton, Menu, MenuItem, MenuTrigger } from '../../ui';
import { AccountCircle, Api, Logout, Settings } from '../../ui/icons';
import { useAuth } from '../../contexts/AuthContext';
import styles from './AppShell.module.css';

export const API_DOCS_PATH = '/swagger-ui';

export function UserMenu() {
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
      <IconButton label={t('shell.user.menu', 'Account menu')} icon={<AccountCircle fontSize="inherit" />} />
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
