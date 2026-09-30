import { useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Input, NumberField, Radio, RadioGroup } from 'react-aria-components';
import { Button, TextField } from '../../../../ui';
import { FieldHelp, FieldLabel } from '../../../../ui/shared/FieldParts';
import fieldStyles from '../../../../ui/shared/field.module.css';
import { ChipList } from '../../shared/parts';
import styles from './SourceForm.module.css';

export interface ListFieldProps {
  label: string;
  description?: ReactNode;
  placeholder?: string;
  items: string[];
  onChange: (items: string[]) => void;
  /** Turns raw input into the stored value ('' rejects it). */
  normalize?: (value: string) => string;
  /** Returns an error message for a value that may not be added. */
  check?: (value: string) => string | null;
  /** Returns advice about a value that is added anyway (shown as a note, never blocks). */
  advise?: (value: string) => string | null;
  /** Error for the list as a whole (e.g. empty). */
  listError?: string | null;
}

/** A list of values with an input and an Add button, shown as removable chips. */
export function ListField({ label, description, placeholder, items, onChange, normalize, check, advise, listError }: ListFieldProps) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [advice, setAdvice] = useState<string | null>(null);
  const value = normalize ? normalize(draft) : draft.trim();

  const add = () => {
    if (!value) return;
    if (items.includes(value)) {
      setError(t('intake.form.errors.duplicate', '“{{value}}” is already in the list', { value }));
      return;
    }
    const problem = check?.(value) ?? null;
    if (problem) {
      setError(problem);
      return;
    }
    onChange([...items, value]);
    setAdvice(advise?.(value) ?? null);
    setDraft('');
    setError(null);
  };

  const shownError = error ?? listError ?? null;
  return (
    <div className={styles.listField}>
      <div className={styles.addRow}>
        <TextField
          className={styles.grow}
          label={label}
          description={description}
          placeholder={placeholder}
          value={draft}
          onChange={(v) => {
            setDraft(v);
            setError(null);
          }}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault();
              add();
            }
          }}
          isInvalid={Boolean(shownError)}
          errorMessage={shownError ?? undefined}
        />
        <Button className={styles.addButton} onPress={add} isDisabled={!value} aria-label={t('intake.form.addTo', 'Add to {{label}}', { label })}>
          {t('intake.form.add', 'Add')}
        </Button>
      </div>
      <p className={styles.hint} role="status">
        {advice}
      </p>
      <ChipList
        items={items}
        label={label}
        onRemove={(item) => onChange(items.filter((i) => i !== item))}
        removeLabel={(item) => t('intake.form.remove', 'Remove {{item}}', { item })}
      />
    </div>
  );
}

export interface IntervalFieldProps {
  label: string;
  description?: ReactNode;
  value: number;
  onChange: (value: number) => void;
  errorMessage?: string | null;
}

/** Whole minutes; range errors come from the form's validation, so typing is never blocked. */
export function IntervalField({ label, description, value, onChange, errorMessage }: IntervalFieldProps) {
  return (
    <NumberField
      className={fieldStyles.field}
      value={Number.isNaN(value) ? undefined : value}
      onChange={(v) => onChange(Number.isNaN(v) ? NaN : Math.round(v))}
      formatOptions={{ maximumFractionDigits: 0, useGrouping: false }}
      isInvalid={Boolean(errorMessage)}
    >
      <FieldLabel label={label} />
      <Input className={`${fieldStyles.control} ${styles.numberInput}`} />
      <FieldHelp description={description} errorMessage={errorMessage ?? undefined} />
    </NumberField>
  );
}

export interface ChoiceOption<T extends string> {
  value: T;
  label: string;
  description?: string;
}

/** A labelled radio group laid out as selectable cards. */
export function ChoiceField<T extends string>({
  label,
  value,
  onChange,
  options,
}: {
  label: string;
  value: T;
  onChange: (value: T) => void;
  options: ChoiceOption<T>[];
}) {
  return (
    <RadioGroup
      className={styles.choice}
      value={value}
      onChange={(v) => onChange(v as T)}
      orientation="horizontal"
    >
      <FieldLabel label={label} />
      <div className={styles.choiceOptions}>
        {options.map((o) => (
          <Radio key={o.value} value={o.value} className={styles.choiceOption}>
            <span className={styles.choiceMark} aria-hidden="true" />
            <span className={styles.choiceText}>
              <span className={styles.choiceLabel}>{o.label}</span>
              {o.description ? (
                <span className={styles.choiceDescription}>{o.description}</span>
              ) : null}
            </span>
          </Radio>
        ))}
      </div>
    </RadioGroup>
  );
}
