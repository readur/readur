import { ToggleButton, ToggleButtonGroup, type Key } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import type { ReadingView } from '../hooks/useReadingView';
import styles from './ViewSwitch.module.css';

export interface ViewSwitchProps {
  view: ReadingView;
  canSplit: boolean;
  onChange: (view: ReadingView) => void;
}

/** Document | Side by side | Text. Side by side is only offered where two panes fit. */
export function ViewSwitch({ view, canSplit, onChange }: ViewSwitchProps) {
  const { t } = useTranslation();
  return (
    <ToggleButtonGroup
      aria-label={t('document.view.label', 'View')}
      selectionMode="single"
      disallowEmptySelection
      selectedKeys={[view]}
      onSelectionChange={(keys: Set<Key>) => {
        const [next] = keys;
        if (next) onChange(next as ReadingView);
      }}
      className={styles.group}
    >
      <ToggleButton id="document" className={styles.option}>
        {t('document.view.document', 'Document')}
      </ToggleButton>
      {canSplit ? (
        <ToggleButton id="split" className={styles.option}>
          {t('document.view.split', 'Side by side')}
        </ToggleButton>
      ) : null}
      <ToggleButton id="text" className={styles.option}>
        {t('document.view.text', 'Text')}
      </ToggleButton>
    </ToggleButtonGroup>
  );
}
