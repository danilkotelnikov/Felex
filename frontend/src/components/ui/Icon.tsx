import { type LucideIcon } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { CSSProperties } from 'react';

interface IconProps {
  icon: LucideIcon;
  size?: number;
  className?: string;
  style?: CSSProperties;
}

export function Icon({ icon: Comp, size = 16, className, style }: IconProps) {
  return (
    <Comp
      size={size}
      strokeWidth={1.5}
      className={cn('shrink-0', className)}
      style={style}
    />
  );
}
