import { useTranslation } from 'react-i18next';
import { Skeleton } from '../../ui';
import styles from './DocumentPage.module.css';

/** Placeholder layout while the document loads. */
export function DocumentPageSkeleton() {
  const { t } = useTranslation();
  return (
    <div className={styles.page} role="status" aria-label={t('document.loading', 'Loading document')}>
      <Skeleton width={160} height={12} />
      <Skeleton width="45%" height={32} />
      <Skeleton height={72} />
      <div className={styles.columns}>
        <Skeleton height={480} />
        <Skeleton lines={10} />
      </div>
    </div>
  );
}
