import type { ReactElement } from 'react';
import { Button as RACButton, Heading } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { Button, IconButton } from '../../ui';
import { CheckCircle, Close, Error as ErrorIcon, Info, Warning } from '../../ui/icons';
import { useNotifications } from '../../contexts/NotificationContext';
import { formatRelativeTime } from '../../lib/relativeTime';
import type { NotificationType } from '../../types/notification';
import styles from './NotificationPanel.module.css';

const TYPE_ICON: Record<NotificationType, ReactElement> = {
  success: <CheckCircle fontSize="inherit" />,
  error: <ErrorIcon fontSize="inherit" />,
  warning: <Warning fontSize="inherit" />,
  info: <Info fontSize="inherit" />,
};

const TYPE_FALLBACK: Record<NotificationType, string> = {
  success: 'Success',
  error: 'Error',
  warning: 'Warning',
  info: 'Info',
};

export interface NotificationPanelProps {
  onClose: () => void;
}

/** Contents of the alerts popover: the NotificationContext list with read/clear actions. */
export function NotificationPanel({ onClose }: NotificationPanelProps) {
  const { t } = useTranslation();
  const { notifications, unreadCount, markAsRead, markAllAsRead, clearNotification, clearAll } =
    useNotifications();

  return (
    <div className={styles.panel}>
      <div className={styles.header}>
        <Heading slot="title" className={styles.title}>
          {t('notifications.title', 'Notifications')}
        </Heading>
        {unreadCount > 0 ? (
          <span className={styles.count}>
            {t('shell.alerts.unread', { count: unreadCount, defaultValue: '{{count}} unread' })}
          </span>
        ) : null}
        <IconButton
          size="sm"
          className={styles.close}
          label={t('shell.alerts.close', 'Close notifications')}
          icon={<Close fontSize="inherit" />}
          onPress={onClose}
        />
      </div>

      {notifications.length === 0 ? (
        <p className={styles.empty}>{t('notifications.noNotifications', 'No notifications')}</p>
      ) : (
        <>
          <div className={styles.actions}>
            <Button size="sm" variant="ghost" onPress={markAllAsRead} isDisabled={unreadCount === 0}>
              {t('notifications.markAllAsRead', 'Mark all as read')}
            </Button>
            <Button size="sm" variant="ghost" onPress={clearAll}>
              {t('notifications.clearAll', 'Clear all')}
            </Button>
          </div>
          <ul className={styles.list} aria-label={t('notifications.title', 'Notifications')}>
            {notifications.map((n) => {
              const typeWord = t(`shell.alerts.type.${n.type}`, TYPE_FALLBACK[n.type]);
              return (
                <li key={n.id} className={styles.item} data-unread={n.read ? undefined : 'true'}>
                  <RACButton
                    className={styles.itemBody}
                    onPress={() => markAsRead(n.id)}
                    aria-label={`${n.read ? '' : `${t('shell.alerts.new', 'New')}: `}${typeWord}: ${n.title}`}
                    aria-describedby={`notification-${n.id}-message`}
                  >
                    <span className={styles.icon} data-type={n.type} aria-hidden="true">
                      {TYPE_ICON[n.type] ?? TYPE_ICON.info}
                    </span>
                    <span className={styles.text}>
                      <span className={styles.itemTitle}>
                        {n.read ? null : <span className={styles.unreadDot} data-unread-dot aria-hidden="true" />}
                        {n.title}
                      </span>
                      <span id={`notification-${n.id}-message`} className={styles.message}>
                        {n.message}
                      </span>
                      <time className={styles.time} dateTime={new Date(n.timestamp).toISOString()}>
                        {formatRelativeTime(n.timestamp)}
                      </time>
                    </span>
                  </RACButton>
                  <IconButton
                    size="sm"
                    className={styles.dismiss}
                    label={t('shell.alerts.dismiss', { title: n.title, defaultValue: 'Dismiss {{title}}' })}
                    icon={<Close fontSize="inherit" />}
                    onPress={() => clearNotification(n.id)}
                  />
                </li>
              );
            })}
          </ul>
        </>
      )}
    </div>
  );
}
