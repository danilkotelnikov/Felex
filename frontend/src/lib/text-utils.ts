/**
 * Text utility functions for proper Cyrillic and Latin text handling.
 */

/**
 * Capitalizes the first character of a string using locale-aware case conversion.
 * Handles Cyrillic characters correctly.
 *
 * @param text - The input string to capitalize
 * @param locale - The locale for case conversion (default: 'ru')
 * @returns The string with first character capitalized, or the original value if falsy
 */
export function capitalizeFirst(text: string | null | undefined, locale = 'ru'): string | null | undefined {
  if (!text) return text;
  return text.charAt(0).toLocaleUpperCase(locale) + text.slice(1);
}
