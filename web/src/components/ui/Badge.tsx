import React from 'react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export interface BadgeProps extends React.HTMLAttributes<HTMLSpanElement> {
  variant?: 'accent' | 'muted' | 'success' | 'warning' | 'danger';
}

export const Badge: React.FC<BadgeProps> = ({ className, variant = 'accent', children, ...props }) => {
  const variants = {
    accent: 'bg-[var(--accent)]/10 text-[var(--accent-text)] border-[var(--accent)]/20',
    muted: 'bg-[var(--surface-hover)] text-[var(--muted)] border-[var(--border)]',
    success: 'bg-[var(--success-bg)] text-[var(--success-text)] border-[var(--success)]/20',
    warning: 'bg-[var(--warning-bg)] text-[var(--warning-text)] border-[var(--warning)]/20',
    danger: 'bg-[var(--danger-bg)] text-[var(--danger-text)] border-[var(--danger)]/20',
  };

  return (
    <span
      className={twMerge(
        clsx(
          'inline-flex items-center gap-1 px-2 py-0.5 text-[11px] font-medium rounded-full border',
          variants[variant],
          className
        )
      )}
      {...props}
    >
      {children}
    </span>
  );
};

export interface DotProps {
  color?: 'accent' | 'muted' | 'success' | 'warning' | 'danger' | 'over';
  pulse?: boolean;
  className?: string;
}

export const Dot: React.FC<DotProps> = ({ color = 'accent', pulse = false, className }) => {
  const colors = {
    accent: 'bg-[var(--accent-text)]',
    muted: 'bg-[var(--muted)]',
    success: 'bg-[var(--success)]',
    warning: 'bg-[var(--warning)]',
    danger: 'bg-[var(--danger)]',
    over: 'bg-[var(--timer-over)]',
  };

  return (
    <span className={twMerge(clsx('relative flex h-2 w-2', className))}>
      {pulse && (
        <span
          className={twMerge(
            clsx('ring-pulse absolute inline-flex h-full w-full rounded-full opacity-75', colors[color])
          )}
        />
      )}
      <span className={twMerge(clsx('relative inline-flex rounded-full h-2 w-2', colors[color]))} />
    </span>
  );
};

