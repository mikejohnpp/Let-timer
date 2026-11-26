import React from 'react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export const Card: React.FC<React.HTMLAttributes<HTMLDivElement>> = ({ className, children, ...props }) => {
  return (
    <div
      className={twMerge(
        clsx(
          'bg-[var(--surface)] border border-[var(--border)] rounded-[var(--radius-card)] p-4 sm:p-5 transition',
          className
        )
      )}
      {...props}
    >
      {children}
    </div>
  );
};

export const Row: React.FC<React.HTMLAttributes<HTMLDivElement>> = ({ className, children, ...props }) => {
  return (
    <div
      className={twMerge(
        clsx(
          'min-h-[44px] flex items-center justify-between px-3 py-2 rounded-[var(--radius-control)] hover:bg-[var(--surface-hover)] transition',
          className
        )
      )}
      {...props}
    >
      {children}
    </div>
  );
};

