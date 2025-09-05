import {
  Tolgee,
  DevTools,
  TolgeeProvider,
  FormatSimple,
  LanguageStorage,
  LanguageDetector,
  TolgeeInstance,
  TranslateParams,
} from '@tolgee/react';

export let tolgee: TolgeeInstance;

const t = (key: string, params?: TranslateParams) => {
  if (!tolgee) {
    console.warn('Tolgee instance is not initialized yet. Call init() first.');

    return key;
  }
  return tolgee.t(key, params);
};

const init = () => {
  tolgee = Tolgee()
    .use(DevTools())
    .use(FormatSimple())
    .use(LanguageStorage())
    .use(LanguageDetector())
    .init({
      defaultLanguage: 'en',
      fallbackLanguage: 'en',

      // for development
      apiUrl: import.meta.env.VITE_TOLGEE_API_URL,
      apiKey: import.meta.env.VITE_TOLGEE_API_KEY,

      // for production
      staticData: {
        en: () => import('../../i18n/en.json'),
        zh: () => import('../../i18n/zh.json'),
      },
    });

  // await tolgee.addActiveNs;

  return tolgee;
};

const i18n = { t, init };

export default i18n;
