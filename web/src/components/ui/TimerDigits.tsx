import React from 'react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export interface TimerDigitsProps {
  digits: string;
  mode?: 'countdown' | 'overtime' | 'elapsed' | 'idle' | 'paused';
  size?: 'display' | 'pill' | 'inline';
  className?: string;
  style?: React.CSSProperties;
  'aria-hidden'?: boolean | 'true' | 'false';
}

export const TimerDigits: React.FC<TimerDigitsProps> = ({
  digits,
  mode = 'countdown',
  size = 'inline',
  className,
  style,
  'aria-hidden': ariaHidden,
}) => {
  const modes = {
    countdown: 'text-[var(--accent-text)]',
    overtime: 'text-[var(--over-text)]',
    elapsed: 'text-[var(--accent-text)]',
    idle: 'text-[var(--muted)]',
    paused: 'text-[var(--muted)]',
  };

  const sizes = {
    display: 'font-light tracking-tighter text-6xl sm:text-8xl md:text-9xl',
    pill: 'font-bold text-xs sm:text-sm',
    inline: 'text-xs',
  };

  return (
    <span
      aria-hidden={ariaHidden}
      style={style}
      className={twMerge(clsx('font-mono tabular-nums', sizes[size], modes[mode], className))}
    >
      {digits}
    </span>
  );
};

