import { Skeleton } from '../Skeleton';
import styles from './BoardTable.module.css';

const WIDTHS = ['72%', '48%', '64%', '56%', '80%', '40%'];

/** Placeholder rows shown while the first page of a table loads. */
export function LoadingRows({ count, label }: { count: number; label: string }) {
  return (
    <div className={styles.loading} role="status" aria-label={label}>
      {Array.from({ length: count }, (_, i) => (
        <div key={i} className={styles.loadingRow} aria-hidden="true">
          <Skeleton width={WIDTHS[i % WIDTHS.length]} height={10} />
        </div>
      ))}
    </div>
  );
}
