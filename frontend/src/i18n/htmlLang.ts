import type { i18n as I18n } from 'i18next';

/** Keeps `<html lang>` equal to the active i18n language, so assistive tech reads the page in it. */
export function syncHtmlLang(instance: I18n, root: HTMLElement = document.documentElement): () => void {
  const apply = (lng?: string) => {
    const lang = instance.resolvedLanguage ?? lng ?? instance.language;
    if (lang && lang !== 'cimode') root.lang = lang;
  };
  apply(instance.language);
  instance.on('languageChanged', apply);
  return () => instance.off('languageChanged', apply);
}
