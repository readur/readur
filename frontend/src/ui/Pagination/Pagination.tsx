import { useTranslation } from 'react-i18next';
import { ChevronLeft, ChevronRight } from '../icons';
import { IconButton } from '../IconButton';
import { Select, SelectItem } from '../Select';
import styles from './Pagination.module.css';

export interface PaginationProps {
  /** 1-based current page. */
  page: number;
  pageSize: number;
  total: number;
  /** Called with the new page (and page size when it changed; the page resets to 1 then). */
  onChange: (page: number, pageSize: number) => void;
  pageSizeOptions?: number[];
}

export function Pagination({ page, pageSize, total, onChange, pageSizeOptions }: PaginationProps) {
  const { t } = useTranslation();
  const pageCount = Math.max(1, Math.ceil(total / pageSize));
  const start = total === 0 ? 0 : (page - 1) * pageSize + 1;
  const end = Math.min(total, page * pageSize);
  const sizeItems = (pageSizeOptions ?? []).map((n) => ({ id: String(n), n }));

  return (
    <nav className={styles.pagination} aria-label={t('ui.pagination.label', 'Pagination')}>
      <span className={styles.readout} aria-live="polite">
        {t('ui.pagination.range', '{{start}}–{{end}} of {{total}}', { start, end, total })}
      </span>
      <div className={styles.controls}>
        {sizeItems.length > 0 ? (
          <div className={styles.size}>
            <Select
              className={styles.sizeSelect}
              aria-label={t('ui.pagination.pageSize', 'Rows per page')}
              items={sizeItems}
              selectedKey={String(pageSize)}
              onSelectionChange={(key) => onChange(1, Number(key))}
            >
              {(item) => <SelectItem id={item.id}>{item.n}</SelectItem>}
            </Select>
          </div>
        ) : null}
        <IconButton
          variant="secondary"
          size="sm"
          label={t('ui.pagination.previous', 'Previous page')}
          icon={<ChevronLeft fontSize="small" />}
          isDisabled={page <= 1}
          onPress={() => onChange(page - 1, pageSize)}
        />
        <IconButton
          variant="secondary"
          size="sm"
          label={t('ui.pagination.next', 'Next page')}
          icon={<ChevronRight fontSize="small" />}
          isDisabled={page >= pageCount}
          onPress={() => onChange(page + 1, pageSize)}
        />
      </div>
    </nav>
  );
}
