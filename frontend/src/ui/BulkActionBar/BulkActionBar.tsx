import type { ReactNode } from 'react';
import { Toolbar } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { Button } from '../Button';
import { Close } from '../icons';
import { cx } from '../shared/FieldParts';
import styles from './BulkActionBar.module.css';

export interface BulkAction {
  id: string;
  label: string;
  icon?: ReactNode;
  tone?: 'default' | 'danger';
  isDisabled?: boolean;
  onPress: () => void;
}

export interface BulkActionBarProps {
  count: number;
  actions: BulkAction[];
  onClear: () => void;
  /** Accessible name of the toolbar. Defaults to "Bulk actions". */
  'aria-label'?: string;
  className?: string;
}

/**
 * Floating toolbar for acting on a selection. The bar is hidden when nothing is selected, but its
 * polite status region stays mounted so the first selection ("1 selected") is announced.
 */
export function BulkActionBar({ count, actions, onClear, className, 'aria-label': ariaLabel }: BulkActionBarProps) {
  const { t } = useTranslation();
  const selectedWord = t('ui.bulk.selected', { defaultValue: 'selected', count });
  return (
    <>
      <span className="visually-hidden" role="status" aria-live="polite">
        {count > 0 ? `${count} ${selectedWord}` : ''}
      </span>
      {count > 0 ? (
        <div className={cx(styles.dock, className)}>
          <Toolbar
            className={styles.bar}
            aria-label={ariaLabel ?? t('ui.bulk.label', { defaultValue: 'Bulk actions' })}
          >
            <span className={styles.count}>
              <span className={styles.number}>{count}</span>{' '}
              <span className={styles.word}>{selectedWord}</span>
            </span>
            <span className={styles.actions}>
              {actions.map((action) => (
                <Button
                  key={action.id}
                  size="sm"
                  variant={action.tone === 'danger' ? 'danger' : 'secondary'}
                  icon={action.icon}
                  isDisabled={action.isDisabled}
                  onPress={action.onPress}
                >
                  {action.label}
                </Button>
              ))}
            </span>
            <Button
              size="sm"
              variant="ghost"
              className={styles.clear}
              icon={
                <span className={styles.clearIcon}>
                  <Close fontSize="inherit" />
                </span>
              }
              onPress={onClear}
            >
              {t('ui.bulk.clear', { defaultValue: 'Clear selection' })}
            </Button>
          </Toolbar>
        </div>
      ) : null}
    </>
  );
}
