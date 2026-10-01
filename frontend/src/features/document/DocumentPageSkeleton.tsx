import { useTranslation } from 'react-i18next';
import { Skeleton } from '../../ui';
import styles from './DocumentPage.module.css';

/** Placeholder in the page's own shape while the document loads. */
export function DocumentPageSkeleton() {
  const { t } = useTranslation();
  return (
    <div className={styles.page} role="status" aria-label={t('document.loading', 'Loading document')}>
      <Skeleton width={160} height={12} />
      <Skeleton width="45%" height={30} />
      <Skeleton width="60%" height={16} />
      <div className={styles.skeletonArea}>
        <Skeleton height={520} />
        <Skeleton lines={12} />
      </div>
    </div>
  );
}
