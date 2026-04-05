import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

/** Merge class names with Tailwind conflict resolution */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** Format number with locale */
export function formatNumber(value: number, decimals = 1): string {
  return value.toLocaleString('ru-RU', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  });
}

/** Format currency */
export function formatCurrency(value: number): string {
  return value.toLocaleString('ru-RU', {
    style: 'currency',
    currency: 'RUB',
    minimumFractionDigits: 0,
    maximumFractionDigits: 2,
  });
}

/** Format percentage */
export function formatPercent(value: number, decimals = 0): string {
  return `${formatNumber(value, decimals)}%`;
}

/** Debounce function */
export function debounce<T extends (...args: unknown[]) => unknown>(
  func: T,
  wait: number
): (...args: Parameters<T>) => void {
  let timeout: ReturnType<typeof setTimeout> | null = null;

  return (...args: Parameters<T>) => {
    if (timeout) clearTimeout(timeout);
    timeout = setTimeout(() => func(...args), wait);
  };
}
