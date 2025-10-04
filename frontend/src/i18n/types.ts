export const supportedLanguages = {
  en: 'English',
  es: 'Español',
} as const;

export type SupportedLanguage = keyof typeof supportedLanguages;

export const defaultLanguage: SupportedLanguage = 'en';
