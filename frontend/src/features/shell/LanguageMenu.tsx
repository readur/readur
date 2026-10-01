import type { Key } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { IconButton, Menu, MenuItem, MenuTrigger } from '../../ui';
import { Check, Language } from '../../ui/icons';
import { supportedLanguages, type SupportedLanguage } from '../../i18n/types';
import styles from './AppShell.module.css';

const LANGUAGES = Object.entries(supportedLanguages) as [SupportedLanguage, string][];

/** Current UI language, reduced to a supported code ("de-DE" → "de"). */
function currentLanguage(lng: string | undefined): SupportedLanguage | undefined {
  const base = (lng ?? '').split('-')[0] as SupportedLanguage;
  return base in supportedLanguages ? base : undefined;
}

export function LanguageMenu() {
  const { t, i18n } = useTranslation();
  const current = currentLanguage(i18n.resolvedLanguage ?? i18n.language);

  return (
    <MenuTrigger>
      <IconButton label={t('shell.language.label', 'Change language')} icon={<Language fontSize="inherit" />} />
      <Menu
        aria-label={t('shell.language.label', 'Change language')}
        selectionMode="single"
        selectedKeys={current ? [current] : []}
        onAction={(key: Key) => void i18n.changeLanguage(String(key))}
      >
        {LANGUAGES.map(([code, name]) => (
          <MenuItem key={code} id={code} textValue={name} lang={code}>
            {({ isSelected }) => (
              <>
                <span className={styles.menuText}>{name}</span>
                {isSelected ? (
                  <span className={styles.menuCheck} aria-hidden="true">
                    <Check fontSize="inherit" />
                  </span>
                ) : null}
              </>
            )}
          </MenuItem>
        ))}
      </Menu>
    </MenuTrigger>
  );
}
