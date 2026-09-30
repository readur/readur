import { useState } from 'react';
import { Dialog as RACDialog } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { IconButton, Popover, PopoverTrigger } from '../../ui';
import { Notifications } from '../../ui/icons';
import { useNotifications } from '../../contexts/NotificationContext';
import { NotificationPanel } from './NotificationPanel';
import styles from './AppShell.module.css';
import panelStyles from './NotificationPanel.module.css';

/** Bell with an unread badge; opens the notification list in a popover. */
export function AlertsButton() {
  const { t } = useTranslation();
  const { unreadCount } = useNotifications();
  const [isOpen, setOpen] = useState(false);

  const label =
    unreadCount > 0
      ? t('shell.alerts.labelUnread', { count: unreadCount, defaultValue: 'Notifications, {{count}} unread' })
      : t('shell.alerts.label', 'Notifications');

  return (
    <PopoverTrigger isOpen={isOpen} onOpenChange={setOpen}>
      <IconButton
        label={label}
        icon={
          <span className={styles.bell}>
            <Notifications fontSize="inherit" />
            {unreadCount > 0 ? (
              <span className={styles.badge}>{unreadCount > 99 ? '99+' : unreadCount}</span>
            ) : null}
          </span>
        }
      />
      <Popover placement="bottom end" className={panelStyles.popover}>
        <RACDialog className={panelStyles.dialog} aria-label={t('notifications.title', 'Notifications')}>
          <NotificationPanel onClose={() => setOpen(false)} />
        </RACDialog>
      </Popover>
    </PopoverTrigger>
  );
}
