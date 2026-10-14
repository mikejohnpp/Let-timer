import React from 'react';
import { clsx } from 'clsx';
import { Check } from 'lucide-react';

export interface StatusGlyphProps {
  status: 'pending' | 'in-progress' | 'paused' | 'completed' | 'cancelled';
  className?: string;
}

export const StatusGlyph: React.FC<StatusGlyphProps> = ({ status, className }) => {
  switch (status) {
    case 'pending':
      return (
        <span
          role="img"
          aria-label="Pending"
          className={clsx('w-4 h-4 rounded-full border border-[var(--border-strong)] inline-block shrink-0', className)}
        />
      );
    case 'in-progress':
      return (
        <span
          role="img"
          aria-label="In progress"
          className={clsx(
            'w-4 h-4 rounded-full border border-[var(--accent-text)] flex items-center justify-center shrink-0',
            className
          )}
        >
          <span className="w-1.5 h-1.5 rounded-full bg-[var(--accent-text)] dot-running" />
        </span>
      );
    case 'paused':
      return (
        <span
          role="img"
          aria-label="Paused"
          className={clsx(
            'w-4 h-4 rounded-full border border-dashed border-[var(--muted)] flex items-center justify-center shrink-0',
            className
          )}
        />
      );
    case 'completed':
      return (
        <span
          role="img"
          aria-label="Completed"
          className={clsx(
            'w-4 h-4 rounded-full border border-[var(--success)] bg-[var(--success)]/15 flex items-center justify-center text-[var(--success-text)] shrink-0',
            className
          )}
        >
          <Check className="w-2.5 h-2.5 stroke-[3]" />
        </span>
      );
    case 'cancelled':
      return (
        <span
          role="img"
          aria-label="Cancelled"
          className={clsx(
            'w-4 h-4 rounded-full border border-[var(--muted)] flex items-center justify-center text-[var(--muted)] shrink-0',
            className
          )}
        >
          <span className="text-[10px] leading-none font-bold">×</span>
        </span>
      );
  }
};

