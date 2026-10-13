import React from 'react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export interface TimerDigitsProps {
  digits: string;
  mode?: 'countdown' | 'overtime' | 'elapsed' | 'idle' | 'paused';
  className?: string;
}

export const TimerDigits: React.FC<TimerDigitsProps> = ({ digits, mode = 'countdown', className }) => {
  const modes = {
    countdown: 'text-[var(--accent-text)]',
    overtime: 'text-[var(--over-text)]',
    elapsed: 'text-[var(--accent-text)]',
    idle: 'text-[var(--muted)]',
    paused: 'text-[var(--muted)]',
  };

  return (
    <span className={twMerge(clsx('font-mono tabular-nums tracking-tight', modes[mode], className))}>
      {digits}
    </span>
  );
};

