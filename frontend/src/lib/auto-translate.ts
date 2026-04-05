import { useTranslation } from 'react-i18next';

type TranslationFallbackOptions = {
  fallback?: string;
} & Record<string, unknown>;

function keyToReadable(key: string): string {
  const lastPart = key.split('.').pop() ?? key;
  return lastPart
    .replace(/_/g, ' ')
    .replace(/\b\w/g, (char) => char.toUpperCase());
}

export function useTranslationWithFallback() {
  const { t: baseT, i18n } = useTranslation();

  const t = (
    key: string,
    optionsOrFallback?: string | TranslationFallbackOptions,
  ): string => {
    const options =
      typeof optionsOrFallback === 'string'
        ? { fallback: optionsOrFallback }
        : optionsOrFallback;
    const { fallback, ...interpolation } = options ?? {};
    const result = baseT(key, {
      ...interpolation,
      defaultValue: '__MISSING__',
    });

    if (result === '__MISSING__') {
      console.warn(`[i18n] Missing translation: ${key}`);
      return fallback ?? keyToReadable(key);
    }

    return result;
  };

  return { t, i18n };
}

export default useTranslationWithFallback;
