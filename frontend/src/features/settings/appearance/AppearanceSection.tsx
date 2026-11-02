import { useContext, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Radio, RadioGroup, Label } from 'react-aria-components';
import { ThemeModeContext, type ThemeModeName } from '../../../theme/useThemeMode';
import { supportedLanguages, type SupportedLanguage } from '../../../i18n/types';
import { SettingGroup } from '../fold/SettingGroup';
import shared from '../shared/shared.module.css';
import styles from './Appearance.module.css';

type ThemeChoice = ThemeModeName | 'system';

/** Same key the theme provider persists to. Removing it hands the choice back to the OS. */
const THEME_STORAGE_KEY = 'themeMode';
const DARK_QUERY = '(prefers-color-scheme: dark)';

function savedTheme(): ThemeModeName | null {
  try {
    const v = window.localStorage.getItem(THEME_STORAGE_KEY);
    return v === 'light' || v === 'dark' ? v : null;
  } catch {
    return null;
  }
}

function systemPrefersDark(): boolean {
  try {
    return typeof window.matchMedia === 'function' && Boolean(window.matchMedia(DARK_QUERY)?.matches);
  } catch {
    return false;
  }
}

const LANGUAGES = Object.entries(supportedLanguages) as [SupportedLanguage, string][];

function baseLanguage(lng: string | undefined): SupportedLanguage {
  const base = (lng ?? 'en').split('-')[0] as SupportedLanguage;
  return base in supportedLanguages ? base : 'en';
}

/** Theme (light, dark or follow the system) and interface language. Both apply immediately. */
export default function AppearanceSection() {
  const { t, i18n } = useTranslation();
  const theme = useContext(ThemeModeContext);
  const [choice, setChoice] = useState<ThemeChoice>(() => savedTheme() ?? 'system');
  const language = baseLanguage(i18n.resolvedLanguage ?? i18n.language);

  const applyTheme = (next: ThemeChoice) => {
    setChoice(next);
    if (!theme) return;
    if (next === 'system') {
      theme.setMode(systemPrefersDark() ? 'dark' : 'light');
      // setMode persists the mode; clear it so the provider keeps following the OS.
      try {
        window.localStorage.removeItem(THEME_STORAGE_KEY);
      } catch {
        /* storage unavailable: the mode still applies for this session */
      }
    } else {
      theme.setMode(next);
    }
  };

  const themeWord = (c: ThemeChoice) =>
    c === 'light'
      ? t('settings.appearance.light', 'Light')
      : c === 'dark'
        ? t('settings.appearance.dark', 'Dark')
        : t('settings.appearance.system', 'System');

  return (
    <div className={shared.stack}>
      <SettingGroup
        id="appearance-theme"
        title={t('settings.appearance.theme', 'Theme')}
        summary={
          choice === 'system' && theme
            ? `${themeWord('system')} · ${themeWord(theme.mode)}`
            : themeWord(choice)
        }
      >
        <RadioGroup value={choice} onChange={(v) => applyTheme(v as ThemeChoice)} className={styles.group}>
          <Label className={styles.legend}>{t('settings.appearance.theme', 'Theme')}</Label>
          {(['light', 'dark', 'system'] as const).map((c) => (
            <Radio key={c} value={c} className={styles.radio}>
              <span className={styles.dot} aria-hidden="true" />
              <span>
                <span className={styles.radioLabel}>{themeWord(c)}</span>
                {c === 'system' ? (
                  <span className={styles.radioHelp}>
                    {t('settings.appearance.systemHelp', 'Follow your device setting')}
                  </span>
                ) : null}
              </span>
            </Radio>
          ))}
        </RadioGroup>
      </SettingGroup>

      <SettingGroup
        id="appearance-language"
        title={t('settings.appearance.language', 'Interface language')}
        summary={supportedLanguages[language]}
      >
        <RadioGroup
          value={language}
          onChange={(v) => void i18n.changeLanguage(v)}
          className={styles.group}
        >
          <Label className={styles.legend}>{t('settings.appearance.language', 'Interface language')}</Label>
          {LANGUAGES.map(([code, name]) => (
            <Radio key={code} value={code} className={styles.radio} lang={code}>
              <span className={styles.dot} aria-hidden="true" />
              <span className={styles.radioLabel}>{name}</span>
            </Radio>
          ))}
        </RadioGroup>
      </SettingGroup>
    </div>
  );
}
