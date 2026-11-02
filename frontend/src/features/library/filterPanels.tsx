import { useContext, useMemo, useState } from 'react';
import { OverlayTriggerStateContext, Radio, RadioGroup } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { Button, Checkbox, SearchField, TextField } from '../../ui';
import type { LabelData } from '../labels';
import type { LibrarySource } from './data';
import { isoDay } from './format';
import { TYPE_GROUPS, TYPE_GROUP_LABELS, type TypeGroup } from './mime';
import { STATUSES, UPLOADED, WATCHED, type OcrStatus } from './urlState';
import styles from './Library.module.css';

const toggle = <T,>(list: readonly T[], item: T): T[] =>
  list.includes(item) ? list.filter((x) => x !== item) : [...list, item];

export const STATUS_WORDS: Record<OcrStatus, { key: string; fallback: string }> = {
  pending: { key: 'library.status.pending', fallback: 'Pending' },
  processing: { key: 'library.status.processing', fallback: 'Processing' },
  completed: { key: 'library.status.completed', fallback: 'Indexed' },
  failed: { key: 'library.status.failed', fallback: 'Failed' },
};

export function TypePanel({ value, onChange }: { value: TypeGroup[]; onChange: (v: TypeGroup[]) => void }) {
  const { t } = useTranslation();
  return (
    <div className={styles.panelList}>
      {TYPE_GROUPS.map((g) => (
        <Checkbox
          key={g}
          label={t(TYPE_GROUP_LABELS[g].key, TYPE_GROUP_LABELS[g].fallback)}
          isSelected={value.includes(g)}
          onChange={() => onChange(toggle(value, g))}
        />
      ))}
    </div>
  );
}

export function LabelPanel({
  labels,
  value,
  onChange,
}: {
  labels: LabelData[];
  value: string[];
  onChange: (v: string[]) => void;
}) {
  const { t } = useTranslation();
  const [filter, setFilter] = useState('');
  const shown = useMemo(() => {
    const q = filter.trim().toLocaleLowerCase();
    return q ? labels.filter((l) => l.name.toLocaleLowerCase().includes(q)) : labels;
  }, [labels, filter]);
  if (labels.length === 0) {
    return <p className={styles.panelEmpty}>{t('library.filters.noLabels', 'No labels yet')}</p>;
  }
  return (
    <div className={styles.panel}>
      <SearchField
        aria-label={t('library.filters.findLabel', 'Find a label')}
        placeholder={t('library.filters.findLabel', 'Find a label')}
        value={filter}
        onChange={setFilter}
        autoFocus
      />
      <div className={styles.panelList}>
        {shown.map((l) => (
          <Checkbox key={l.id} label={l.name} isSelected={value.includes(l.id)} onChange={() => onChange(toggle(value, l.id))} />
        ))}
        {shown.length === 0 ? (
          <p className={styles.panelEmpty}>{t('library.filters.noLabelMatch', 'No matching labels')}</p>
        ) : null}
      </div>
    </div>
  );
}

/** Closes the surrounding filter popover after a single-choice pick. */
function useClosePopover() {
  const state = useContext(OverlayTriggerStateContext);
  return () => state?.close();
}

export function StatusPanel({ value, onChange }: { value: OcrStatus | null; onChange: (v: OcrStatus | null) => void }) {
  const { t } = useTranslation();
  const close = useClosePopover();
  return (
    <RadioGroup
      className={styles.panelList}
      aria-label={t('library.filters.status', 'Status')}
      value={value ?? 'any'}
      onChange={(v) => {
        onChange(v === 'any' ? null : (v as OcrStatus));
        close();
      }}
    >
      <Radio value="any" className={styles.radio}>
        {t('library.filters.any', 'Any')}
      </Radio>
      {STATUSES.map((s) => (
        <Radio key={s} value={s} className={styles.radio}>
          {t(STATUS_WORDS[s].key, STATUS_WORDS[s].fallback)}
        </Radio>
      ))}
    </RadioGroup>
  );
}

const KINDS = [UPLOADED, WATCHED];

/**
 * Uploads, the watch folder and each connection. The API cannot combine the first two with
 * specific connections, so picking one of those clears the connections and the other way round.
 */
export function SourcePanel({
  sources,
  value,
  onChange,
}: {
  sources: LibrarySource[];
  value: string[];
  onChange: (v: string[]) => void;
}) {
  const { t } = useTranslation();
  const kinds = value.filter((v) => KINDS.includes(v));
  const connections = value.filter((v) => !KINDS.includes(v));
  const kind = (id: string, label: string) => (
    <Checkbox label={label} isSelected={kinds.includes(id)} onChange={() => onChange(toggle(kinds, id))} />
  );
  return (
    <div className={styles.panelList}>
      {kind(UPLOADED, t('library.source.upload', 'Upload'))}
      {kind(WATCHED, t('library.source.watch', 'Watch folder'))}
      {sources.map((s) => (
        <Checkbox
          key={s.id}
          label={s.name}
          isSelected={connections.includes(s.id)}
          onChange={() => onChange(toggle(connections, s.id))}
        />
      ))}
    </div>
  );
}

export const ADDED_PRESETS = [
  { id: '7d', days: 7, key: 'library.added.7d', fallback: 'Last 7 days' },
  { id: '30d', days: 30, key: 'library.added.30d', fallback: 'Last 30 days' },
  { id: '1y', days: 365, key: 'library.added.1y', fallback: 'Last year' },
] as const;

export function presetFrom(days: number, now = new Date()): string {
  const d = new Date(now);
  d.setDate(d.getDate() - days);
  return isoDay(d);
}

export function AddedPanel({
  from,
  to,
  onChange,
}: {
  from: string | null;
  to: string | null;
  onChange: (range: { from: string | null; to: string | null }) => void;
}) {
  const { t } = useTranslation();
  const close = useClosePopover();
  const invalid = Boolean(from && to && from > to);
  return (
    <div className={styles.panel}>
      <div className={styles.presets}>
        {ADDED_PRESETS.map((p) => (
          <Button
            key={p.id}
            size="sm"
            variant="secondary"
            onPress={() => {
              onChange({ from: presetFrom(p.days), to: null });
              close();
            }}
          >
            {t(p.key, p.fallback)}
          </Button>
        ))}
      </div>
      <div className={styles.dateRow}>
        <TextField
          type="date"
          label={t('library.added.from', 'From')}
          value={from ?? ''}
          onChange={(v) => onChange({ from: v || null, to })}
        />
        <TextField
          type="date"
          label={t('library.added.to', 'To')}
          value={to ?? ''}
          onChange={(v) => onChange({ from, to: v || null })}
          isInvalid={invalid}
          errorMessage={t('library.added.order', 'The end date is before the start date')}
        />
      </div>
    </div>
  );
}
