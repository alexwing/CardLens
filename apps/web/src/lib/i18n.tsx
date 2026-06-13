import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react';
import type { ReactNode } from 'react';
import es, { type TranslationKey } from './locales/es';
import en from './locales/en';

/** Idiomas soportados. Anadir uno = anadir su diccionario y este literal. */
export type Locale = 'es' | 'en';

const DICTIONARIES: Record<Locale, Record<TranslationKey, string>> = { es, en };
const STORAGE_KEY = 'cardlens.lang';

/** Locale BCP-47 para Intl (fechas, monedas). */
export function intlLocale(locale: Locale): string {
  return locale === 'en' ? 'en-US' : 'es-ES';
}

function detectLocale(): Locale {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === 'es' || saved === 'en') return saved;
  } catch {
    /* localStorage no disponible */
  }
  const nav = (typeof navigator !== 'undefined' ? navigator.language : 'es') || 'es';
  return nav.toLowerCase().startsWith('en') ? 'en' : 'es';
}

type TParams = Record<string, string | number>;

interface I18nContextValue {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  /** Traduce una clave, interpolando {param} si se pasan params. */
  t: (key: TranslationKey, params?: TParams) => string;
}

const I18nContext = createContext<I18nContextValue | null>(null);

export function LanguageProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(detectLocale);

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  const setLocale = useCallback((next: Locale) => {
    setLocaleState(next);
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      /* localStorage no disponible */
    }
  }, []);

  const t = useCallback(
    (key: TranslationKey, params?: TParams): string => {
      let text = DICTIONARIES[locale][key] ?? DICTIONARIES.es[key] ?? key;
      if (params) {
        for (const [name, value] of Object.entries(params)) {
          text = text.replace(new RegExp(`\\{${name}\\}`, 'g'), String(value));
        }
      }
      return text;
    },
    [locale],
  );

  const value = useMemo<I18nContextValue>(() => ({ locale, setLocale, t }), [locale, setLocale, t]);
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

/** Hook para traducir y cambiar de idioma. Requiere <LanguageProvider> arriba. */
export function useT(): I18nContextValue {
  const ctx = useContext(I18nContext);
  if (!ctx) {
    throw new Error('useT debe usarse dentro de <LanguageProvider>');
  }
  return ctx;
}
