import { useState, type FormEvent, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import type { Key } from 'react-aria-components';
import { Button, Select, SelectItem, TextField } from '../../../ui';
import { Notice } from '../shared/Notice';
import { formatValue, parseField, validateField, type FieldSpec } from './fields';
import type { SettingKey, SettingsPatch, SettingsValues } from './settingsModel';
import styles from './form.module.css';

export interface SettingsFormProps {
  fields: FieldSpec[];
  values: SettingsValues;
  onSave: (patch: SettingsPatch) => Promise<void>;
  isDisabled?: boolean;
  /** Rendered above the fields (e.g. switches that save on their own). */
  before?: ReactNode;
  /** Rendered below the fields. */
  after?: ReactNode;
}

type Draft = Partial<Record<SettingKey, string>>;

/**
 * Fields that are saved together. Save and Cancel appear only while a field differs from the
 * saved value; Save validates every field inline first and sends only the changed keys.
 */
export function SettingsForm({ fields, values, onSave, isDisabled, before, after }: SettingsFormProps) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<Draft>({});
  const [errors, setErrors] = useState<Partial<Record<SettingKey, string>>>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const current = (f: FieldSpec) => draft[f.key] ?? formatValue(f, values[f.key]);
  const changed = fields.filter((f) => draft[f.key] !== undefined && draft[f.key] !== formatValue(f, values[f.key]));
  const dirty = changed.length > 0;

  const setValue = (f: FieldSpec, raw: string) => {
    setDraft((d) => ({ ...d, [f.key]: raw }));
    if (errors[f.key]) setErrors((e) => ({ ...e, [f.key]: validateField(f, raw, t) ?? undefined }));
  };

  const cancel = () => {
    setDraft({});
    setErrors({});
    setFormError(null);
  };

  const submit = async (e?: FormEvent) => {
    e?.preventDefault();
    const nextErrors: Partial<Record<SettingKey, string>> = {};
    for (const f of changed) {
      const err = validateField(f, current(f), t);
      if (err) nextErrors[f.key] = err;
    }
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0) return;
    const patch: SettingsPatch = {};
    for (const f of changed) (patch as Record<string, unknown>)[f.key] = parseField(f, current(f));
    setSaving(true);
    setFormError(null);
    try {
      await onSave(patch);
      setDraft({});
    } catch (err) {
      setFormError(err instanceof Error ? err.message : t('settings.messages.settingsUpdateFailed'));
    } finally {
      setSaving(false);
    }
  };

  return (
    <form className={styles.form} onSubmit={submit} noValidate>
      {before ? <div className={styles.switches}>{before}</div> : null}
      {fields.length > 0 ? (
        <div className={styles.fields}>
          {fields.map((f) =>
            f.kind === 'select' ? (
              <Select
                key={f.key}
                label={t(f.label)}
                description={f.help ? t(f.help) : undefined}
                selectedKey={current(f)}
                onSelectionChange={(k: Key | null) => k !== null && setValue(f, String(k))}
                isDisabled={isDisabled || saving}
              >
                {f.options.map((o) => (
                  <SelectItem key={String(o.value)} id={String(o.value)} textValue={t(o.label, o.fallback)}>
                    {t(o.label, o.fallback)}
                  </SelectItem>
                ))}
              </Select>
            ) : (
              <TextField
                key={f.key}
                label={t(f.label)}
                description={f.help ? t(f.help) : undefined}
                type="number"
                inputMode="decimal"
                value={current(f)}
                onChange={(v) => setValue(f, v)}
                isDisabled={isDisabled || saving}
                isInvalid={Boolean(errors[f.key])}
                errorMessage={errors[f.key]}
                validationBehavior="aria"
              />
            ),
          )}
        </div>
      ) : null}
      {after}
      {formError ? <Notice tone="danger">{formError}</Notice> : null}
      {dirty ? (
        <div className={styles.actions}>
          <Button variant="ghost" onPress={cancel} isDisabled={saving}>
            {t('common.actions.cancel')}
          </Button>
          <Button variant="primary" type="submit" isPending={saving}>
            {t('common.actions.save')}
          </Button>
        </div>
      ) : null}
    </form>
  );
}
