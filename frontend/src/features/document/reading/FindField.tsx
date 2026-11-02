import { Button, Input, SearchField } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { ChevronLeft, ChevronRight, Close, Search } from '../../../ui/icons';
import styles from './OcrTextPanel.module.css';

export interface FindFieldProps {
  value: string;
  onChange: (value: string) => void;
  /** Zero-based position of the current match. */
  current: number;
  total: number;
  onStep: (delta: 1 | -1) => void;
}

/**
 * Find-in-text as one control: the field, the "3 of 7" count and previous / next live in the
 * same box. Enter goes to the next match, Shift+Enter to the previous one, Esc clears.
 */
export function FindField({ value, onChange, current, total, onStep }: FindFieldProps) {
  const { t } = useTranslation();
  const hasQuery = value.trim() !== '';
  const readout = !hasQuery
    ? ''
    : total === 0
      ? t('document.text.noMatches', 'No matches')
      : t('document.text.matchPosition', { current: current + 1, total, defaultValue: '{{current}} of {{total}}' });

  return (
    <SearchField
      aria-label={t('document.text.find', 'Find in text')}
      value={value}
      onChange={onChange}
      className={styles.find}
      data-empty={hasQuery && total === 0 ? 'true' : undefined}
    >
      <span className={styles.findIcon} aria-hidden="true">
        <Search fontSize="inherit" />
      </span>
      <Input
        className={styles.findInput}
        placeholder={t('document.text.findPlaceholder', 'Find in text…')}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault();
            onStep(e.shiftKey ? -1 : 1);
          }
        }}
      />
      <span className={styles.readout} role="status" aria-live="polite">
        {readout}
      </span>
      {hasQuery ? (
        <Button className={styles.findButton} aria-label={t('ui.clear', 'Clear')}>
          <Close fontSize="inherit" />
        </Button>
      ) : null}
      <span className={styles.findDivider} aria-hidden="true" />
      <Button
        slot={null}
        className={styles.findButton}
        aria-label={t('document.text.previousMatch', 'Previous match')}
        isDisabled={total === 0}
        onPress={() => onStep(-1)}
      >
        <ChevronLeft fontSize="inherit" />
      </Button>
      <Button
        slot={null}
        className={styles.findButton}
        aria-label={t('document.text.nextMatch', 'Next match')}
        isDisabled={total === 0}
        onPress={() => onStep(1)}
      >
        <ChevronRight fontSize="inherit" />
      </Button>
    </SearchField>
  );
}
