import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { CheckCircle, Close, Error as ErrorIcon, Info, Warning } from '../icons';
import { IconButton } from '../IconButton';
import styles from './Notice.module.css';

export type NoticeTone = 'info' | 'ok' | 'warning' | 'danger';

export interface NoticeProps {
  tone?: NoticeTone;
  title?: ReactNode;
  children?: ReactNode;
  /** Trailing action, typically a ghost Button or a link. */
  action?: ReactNode;
  onDismiss?: () => void;
  /** Live region role. Default: danger → `alert`, others → `status`; `off` renders none. */
  live?: 'alert' | 'status' | 'off';
  /** Visually hidden word read before the title (e.g. "Error:"). */
  prefix?: string;
}

const ICONS = { info: Info, ok: CheckCircle, warning: Warning, danger: ErrorIcon } as const;

/** Inline message card. The tone is carried by a stripe, an icon and the words, never colour alone. */
export function Notice({ tone = 'info', title, children, action, onDismiss, live, prefix }: NoticeProps) {
  const { t } = useTranslation();
  const Icon = ICONS[tone];
  const role = live === 'off' ? undefined : (live ?? (tone === 'danger' ? 'alert' : 'status'));
  return (
    <div className={styles.notice} data-tone={tone} role={role}>
      <span className={styles.icon} aria-hidden="true">
        <Icon fontSize="inherit" />
      </span>
      <div className={styles.body}>
        {prefix || title ? (
          <p className={styles.title}>
            {prefix ? <span className="visually-hidden">{prefix} </span> : null}
            {title}
          </p>
        ) : null}
        {children ? <div className={styles.text}>{children}</div> : null}
      </div>
      {action ? <div className={styles.action}>{action}</div> : null}
      {onDismiss ? (
        <IconButton size="sm" label={t('common.actions.close', 'Close')} icon={<Close fontSize="inherit" />} onPress={onDismiss} />
      ) : null}
    </div>
  );
}
