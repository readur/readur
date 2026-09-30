import { createContext, useContext, useMemo, useState, type ReactNode } from 'react';
import {
  Button,
  Text,
  UNSTABLE_Toast as RACToast,
  UNSTABLE_ToastContent as ToastContent,
  UNSTABLE_ToastQueue as ToastQueue,
  UNSTABLE_ToastRegion as ToastRegion,
} from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { Close, Error as ErrorIcon, Info, CheckCircle } from '../icons';
import { cx } from '../shared/FieldParts';
import styles from './Toast.module.css';

export type ToastTone = 'info' | 'success' | 'danger';

export interface ToastOptions {
  title: string;
  description?: string;
  tone?: ToastTone;
  /** Auto-dismiss delay in ms. Default 5000. */
  timeout?: number;
}

export interface ToastApi {
  show: (options: ToastOptions) => void;
}

interface ToastContentValue {
  title: string;
  description?: string;
  tone: ToastTone;
}

const DEFAULT_TIMEOUT = 5000;

const TONE_DEFAULT = { info: 'Info:', success: 'Success:', danger: 'Error:' } as const;

const TONE_ICON = { info: Info, success: CheckCircle, danger: ErrorIcon } as const;

const ToastContext = createContext<ToastApi | null>(null);

/** Mount once near the app root. Renders the toast region (bottom-right). */
export function ToastProvider({ children }: { children: ReactNode }) {
  const { t } = useTranslation();
  const [queue] = useState(() => new ToastQueue<ToastContentValue>({ maxVisibleToasts: 5 }));
  const api = useMemo<ToastApi>(
    () => ({
      show: ({ title, description, tone = 'info', timeout = DEFAULT_TIMEOUT }) => {
        queue.add({ title, description, tone }, { timeout });
      },
    }),
    [queue],
  );

  return (
    <ToastContext.Provider value={api}>
      {children}
      <ToastRegion queue={queue} className={styles.region} aria-label={t('ui.notifications', 'Notifications')}>
        {({ toast }) => (
          <RACToast toast={toast} className={cx(styles.toast, styles[toast.content.tone])}>
            <span className={styles.toneIcon} aria-hidden="true">
              {(() => {
                const Icon = TONE_ICON[toast.content.tone];
                return <Icon fontSize="small" />;
              })()}
            </span>
            <ToastContent className={styles.content} role={toast.content.tone === 'danger' ? 'alert' : 'status'}>
              <Text slot="title" className={styles.title}>
                <span className="visually-hidden">{t(`ui.toast.${toast.content.tone}`, TONE_DEFAULT[toast.content.tone])} </span>
                {toast.content.title}
              </Text>
              {toast.content.description ? (
                <Text slot="description" className={styles.description}>
                  {toast.content.description}
                </Text>
              ) : null}
            </ToastContent>
            <Button slot="close" className={styles.close} aria-label={t('ui.close', 'Close')}>
              <Close fontSize="small" />
            </Button>
          </RACToast>
        )}
      </ToastRegion>
    </ToastContext.Provider>
  );
}

/** Returns `{ show }`. Outside a provider it is a no-op so isolated component tests keep working. */
export function useToast(): ToastApi {
  return useContext(ToastContext) ?? NOOP;
}

const NOOP: ToastApi = { show: () => undefined };
