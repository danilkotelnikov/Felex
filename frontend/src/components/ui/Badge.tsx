import { cn } from '@/lib/utils';

interface BadgeProps {
  variant?: 'default' | 'secondary' | 'success' | 'warning' | 'error' | 'info';
  children: React.ReactNode;
  className?: string;
}

const variants = {
  default: 'bg-[--bg-hover] text-[--text-secondary]',
  secondary: 'bg-[--bg-surface] text-[--text-secondary] border border-[--border]',
  success: 'bg-[--status-ok-bg] text-[--status-ok]',
  warning: 'bg-[--status-warn-bg] text-[--status-warn]',
  error: 'bg-[--status-error-bg] text-[--status-error]',
  info: 'bg-[--status-info-bg] text-[--status-info]',
};

export function Badge({ variant = 'default', children, className }: BadgeProps) {
  return (
    <span
      className={cn(
        'inline-flex items-center px-2 py-0.5 rounded text-xs font-medium',
        variants[variant],
        className
      )}
    >
      {children}
    </span>
  );
}
