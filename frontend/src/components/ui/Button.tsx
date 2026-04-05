import { forwardRef, type ButtonHTMLAttributes } from 'react';
import { cn } from '@/lib/utils';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'default' | 'ghost' | 'outline' | 'danger';
  size?: 'sm' | 'md' | 'lg';
}

const variants = {
  default: 'bg-[--accent] text-[--text-inverse] hover:bg-[--accent-hover]',
  ghost: 'text-[--text-secondary] hover:bg-[--bg-hover] hover:text-[--text-primary]',
  outline: 'border border-[--border] hover:bg-[--bg-hover]',
  danger: 'text-[--status-error] hover:bg-[--status-error-bg]',
};

const sizes = {
  sm: 'h-6 px-2 text-xs gap-1',
  md: 'h-8 px-3 text-sm gap-1.5',
  lg: 'h-10 px-4 text-sm gap-2',
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ variant = 'default', size = 'md', className, children, ...props }, ref) => {
    return (
      <button
        ref={ref}
        className={cn(
          'inline-flex items-center justify-center rounded-[--radius-sm]',
          'font-medium transition-colors duration-100',
          'focus-visible:outline-none focus-visible:ring-2',
          'focus-visible:ring-[--border-focus] focus-visible:ring-offset-1',
          'disabled:opacity-40 disabled:pointer-events-none',
          variants[variant],
          sizes[size],
          className
        )}
        {...props}
      >
        {children}
      </button>
    );
  }
);

Button.displayName = 'Button';
