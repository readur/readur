import { useTranslation } from 'react-i18next';
import { useAuth } from '../../../contexts/AuthContext';
import { Facts } from '../shared/Facts';
import shared from '../shared/shared.module.css';
import { ChangePasswordForm } from './ChangePasswordForm';

/** The signed-in user's own account: who they are signed in as, and a password change. */
export default function AccountSection() {
  const { t } = useTranslation();
  const { user } = useAuth();
  return (
    <div className={shared.stack}>
      {user ? (
        <Facts
          items={[
            { label: t('settings.userManagement.tableHeaders.username', 'Username'), value: user.username },
            { label: t('settings.userManagement.tableHeaders.email', 'Email'), value: user.email },
          ]}
        />
      ) : null}
      <ChangePasswordForm />
    </div>
  );
}
