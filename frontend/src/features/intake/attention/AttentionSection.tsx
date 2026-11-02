import type { ComponentType } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { ToggleButton, ToggleButtonGroup, type Key } from 'react-aria-components';
import { sharedStyles } from '../shared/parts';
import { CleanupPanel } from './CleanupPanel';
import { DuplicatesPanel } from './DuplicatesPanel';
import { FailedOcrPanel } from './FailedOcrPanel';
import { LowConfidencePanel } from './LowConfidencePanel';
import styles from './Attention.module.css';

export const ATTENTION_VIEWS = ['failed', 'lowConfidence', 'duplicates', 'cleanup'] as const;
export type AttentionView = (typeof ATTENTION_VIEWS)[number];

const VIEWS: Record<AttentionView, ComponentType> = {
  failed: FailedOcrPanel,
  lowConfidence: LowConfidencePanel,
  duplicates: DuplicatesPanel,
  cleanup: CleanupPanel,
};

const parseView = (value: string | null): AttentionView =>
  (ATTENTION_VIEWS as readonly string[]).includes(value ?? '') ? (value as AttentionView) : 'failed';

/** Documents that need a decision: failed OCR, low confidence, duplicates and bulk cleanup. */
export function AttentionSection() {
  const { t } = useTranslation();
  const [params, setParams] = useSearchParams();
  const view = parseView(params.get('view'));
  const labels: Record<AttentionView, string> = {
    failed: t('intake.attention.view.failed', 'Failed OCR'),
    lowConfidence: t('intake.attention.view.lowConfidence', 'Low confidence'),
    duplicates: t('intake.attention.view.duplicates', 'Duplicates'),
    cleanup: t('intake.attention.view.cleanup', 'Cleanup'),
  };

  const select = (keys: Set<Key>) => {
    const next = parseView(String([...keys][0] ?? ''));
    setParams(
      (prev) => {
        const out = new URLSearchParams(prev);
        out.set('section', 'attention');
        out.set('view', next);
        return out;
      },
      { replace: true },
    );
  };

  const View = VIEWS[view];
  return (
    <div className={sharedStyles.section}>
      <ToggleButtonGroup
        className={styles.segments}
        aria-label={t('intake.attention.filter', 'Show')}
        selectionMode="single"
        disallowEmptySelection
        selectedKeys={[view]}
        onSelectionChange={select}
      >
        {ATTENTION_VIEWS.map((id) => (
          <ToggleButton key={id} id={id} className={styles.segment}>
            {labels[id]}
          </ToggleButton>
        ))}
      </ToggleButtonGroup>
      <View />
    </div>
  );
}
