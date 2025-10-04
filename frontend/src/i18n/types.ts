export const supportedLanguages = {
  en: 'English',
  es: 'Español',
  de: 'Deutsch',
  fr: 'Français',
} as const;

export type SupportedLanguage = keyof typeof supportedLanguages;

export const defaultLanguage: SupportedLanguage = 'en';
