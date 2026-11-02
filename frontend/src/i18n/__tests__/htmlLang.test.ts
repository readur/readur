import { afterEach, describe, expect, it } from 'vitest';
import i18next from 'i18next';
import { syncHtmlLang } from '../htmlLang';

const make = async (lng: string) => {
  const instance = i18next.createInstance();
  await instance.init({
    lng,
    fallbackLng: 'en',
    supportedLngs: ['en', 'de', 'es', 'fr'],
    resources: { en: { translation: {} }, de: { translation: {} }, es: { translation: {} }, fr: { translation: {} } },
  });
  return instance;
};

describe('syncHtmlLang', () => {
  afterEach(() => {
    document.documentElement.lang = 'en';
  });

  it('sets <html lang> to the current language', async () => {
    const instance = await make('de');
    const stop = syncHtmlLang(instance);
    expect(document.documentElement.lang).toBe('de');
    stop();
  });

  it('follows language changes and stops when asked', async () => {
    const instance = await make('en');
    const stop = syncHtmlLang(instance);
    await instance.changeLanguage('fr');
    expect(document.documentElement.lang).toBe('fr');
    stop();
    await instance.changeLanguage('es');
    expect(document.documentElement.lang).toBe('fr');
  });
});
