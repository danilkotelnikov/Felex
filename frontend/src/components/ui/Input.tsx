import { forwardRef, type InputHTMLAttributes } from 'react';
import { cn } from '@/lib/utils';

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  error?: boolean;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ className, error, ...props }, ref) => {
    return (
      <input
        ref={ref}
        className={cn(
          'flex h-8 w-full rounded-[--radius-sm] px-2',
          'bg-[--bg-base] border border-[--border]',
          'text-sm text-[--text-primary]',
          'placeholder:text-[--text-disabled]',
          'focus:outline-none focus:ring-1 focus:ring-[--border-focus]',
          'disabled:opacity-50 disabled:cursor-not-allowed',
          error && 'border-[--status-error] focus:ring-[--status-error]',
          className
        )}
        {...props}
      />
    );
  }
);

Input.displayName = 'Input';
