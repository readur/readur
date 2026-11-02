import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import styles from './Auth.module.css';

/** Centred public page: wordmark on top, content below, optional footer line. */
export function AuthLayout({ children, footer }: { children: ReactNode; footer?: ReactNode }) {
  const { t } = useTranslation();
  return (
    <main className={styles.page}>
      <div className={styles.column}>
        <div className={styles.brand}>
          <img src="/readur-64.png" alt="" width={28} height={28} className={styles.logo} />
          <span className={styles.wordmark}>{t('auth.login.wordmark', 'Readur')}</span>
        </div>
        {children}
        {footer ? <p className={styles.footer}>{footer}</p> : null}
      </div>
    </main>
  );
}
