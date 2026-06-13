import { useT } from '../lib/i18n';
import type { Locale } from '../lib/i18n';

/** Selector de idioma ES / EN. */
export default function LanguageToggle() {
  const { locale, setLocale, t } = useT();
  const locales: Locale[] = ['es', 'en'];

  return (
    <div className="lang-toggle" role="group" aria-label={t('lang.ariaLabel')}>
      {locales.map((code) => (
        <button
          key={code}
          type="button"
          className={`lang-option${locale === code ? ' active' : ''}`}
          aria-pressed={locale === code}
          onClick={() => setLocale(code)}
        >
          {code.toUpperCase()}
        </button>
      ))}
    </div>
  );
}
