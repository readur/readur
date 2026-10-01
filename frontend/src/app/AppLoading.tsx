import { useTranslation } from 'react-i18next';
import { Skeleton } from '../ui';
import styles from './AppLoading.module.css';

/** Shown while the session is being restored. */
export function AppLoading() {
  const { t } = useTranslation();
  return (
    <div className={styles.loading}>
      <Skeleton width={160} height={12} label={t('shell.loading', 'Loading')} />
    </div>
  );
}
