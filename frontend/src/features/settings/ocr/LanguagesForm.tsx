import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '../../../ui';
import LanguageSelector from '../../../components/LanguageSelector';
import type { SettingsPatch, SettingsValues } from '../form/settingsModel';
import { Notice } from '../shared/Notice';
import styles from '../form/form.module.css';

interface Draft {
  languages: string[];
  primary: string;
}

const sameList = (a: string[], b: string[]) => a.length === b.length && a.every((x, i) => x === b[i]);

/** OCR languages with primary: edited as a draft, saved together (preferred, primary and legacy `ocr_language`). */
export function LanguagesForm({ values, onSave }: { values: SettingsValues; onSave: (p: SettingsPatch) => Promise<void> }) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<Draft | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const languages = draft?.languages ?? values.preferred_languages;
  const primary = draft?.primary ?? values.primary_language;
  const dirty =
    draft !== null && (!sameList(draft.languages, values.preferred_languages) || draft.primary !== values.primary_language);

  const save = async () => {
    if (!draft) return;
    const main = draft.primary || draft.languages[0] || 'eng';
    setSaving(true);
    setError(null);
    try {
      await onSave({ preferred_languages: draft.languages, primary_language: main, ocr_language: main });
      setDraft(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : t('settings.messages.settingsUpdateFailed'));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className={styles.form}>
      <p className={styles.help}>{t('settings.general.ocrConfiguration.description')}</p>
      <LanguageSelector
        selectedLanguages={languages}
        primaryLanguage={primary}
        onLanguagesChange={(next, nextPrimary) => setDraft({ languages: next, primary: nextPrimary ?? next[0] ?? '' })}
        disabled={saving}
        showPrimarySelector
      />
      {error ? <Notice tone="danger">{error}</Notice> : null}
      {dirty ? (
        <div className={styles.actions}>
          <Button
            variant="ghost"
            onPress={() => {
              setDraft(null);
              setError(null);
            }}
            isDisabled={saving}
          >
            {t('common.actions.cancel')}
          </Button>
          <Button variant="primary" onPress={save} isPending={saving}>
            {t('common.actions.save')}
          </Button>
        </div>
      ) : null}
    </div>
  );
}
