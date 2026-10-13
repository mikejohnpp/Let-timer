import React from 'react';
import * as ProgressPrimitive from '@radix-ui/react-progress';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export interface ProgressLineProps {
  value: number; // 0 to 100
  overtime?: boolean;
  className?: string;
}

export const ProgressLine: React.FC<ProgressLineProps> = ({ value, overtime = false, className }) => {
  const clamped = Math.min(100, Math.max(0, value));

  return (
    <ProgressPrimitive.Root
      value={clamped}
      className={twMerge(
        clsx('relative h-[3px] w-full overflow-hidden rounded-full bg-[var(--border)]', className)
      )}
    >
      <ProgressPrimitive.Indicator
        className={twMerge(
          clsx(
            'h-full w-full flex-1 transition-all duration-300',
            overtime ? 'bg-[var(--timer-over)]' : 'bg-[var(--accent-text)]'
          )
        )}
        style={{ transform: `translateX(-${100 - clamped}%)` }}
      />
    </ProgressPrimitive.Root>
  );
};

