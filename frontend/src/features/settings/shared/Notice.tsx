import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { IconButton } from '../../../ui';
import { Close, Error as ErrorIcon, Info, Warning } from '../../../ui/icons';
import styles from './shared.module.css';

export interface NoticeProps {
  tone?: 'info' | 'warning' | 'danger';
  title?: ReactNode;
  children?: ReactNode;
  onDismiss?: () => void;
  action?: ReactNode;
}

const ICONS = { info: Info, warning: Warning, danger: ErrorIcon } as const;

/** Inline message block. Danger notices are announced assertively; the tone is also an icon. */
export function Notice({ tone = 'info', title, children, onDismiss, action }: NoticeProps) {
  const { t } = useTranslation();
  const Icon = ICONS[tone];
  return (
    <div className={styles.notice} data-tone={tone} role={tone === 'danger' ? 'alert' : 'status'}>
      <span className={styles.noticeIcon} aria-hidden="true">
        <Icon fontSize="inherit" />
      </span>
      <div className={styles.noticeBody}>
        {title ? <p className={styles.noticeTitle}>{title}</p> : null}
        {children ? <div>{children}</div> : null}
        {action ? <div className={styles.noticeAction}>{action}</div> : null}
      </div>
      {onDismiss ? (
        <IconButton
          size="sm"
          label={t('common.actions.close', 'Close')}
          icon={<Close fontSize="inherit" />}
          onPress={onDismiss}
        />
      ) : null}
    </div>
  );
}
